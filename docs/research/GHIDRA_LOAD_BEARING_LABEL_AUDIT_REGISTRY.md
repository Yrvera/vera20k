# Ghidra Load-Bearing Label Audit Registry — gamemd.exe

**Date:** 2026-05-30
**Program:** gamemd.exe (PE x86 32-bit, image base `0x00400000`, 9911 functions, 40220 symbols)
**Scope:** High-risk labels only — vtables/slots, lifecycle/destructors, schedulers, offset-owner
functions, type/INI lookup, cell/bridge substrate. Labels treated as navigation hints, not truth.
**Method:** 6 discovery agents (one per category) → 36 deep per-symbol verifications (bytes + body +
`get_function_callers`/`get_xrefs_to` + vtable-slot `read_memory` + active-YR reachability) → one
serialized writer that re-verified each rename from the binary, applied it, read it back, and saved.
**Authority order:** binary → Ghidra → docs. Burden of proof defaulted to *label is wrong* unless
airtight. Extends `LABEL_AUDIT_LOG.md` (does not redo it).

Statuses: **VERIFIED** (label matches body+role) · **RENAMED** (applied this pass) · **MISLEADING**
(name implies wrong role — proposal only when not certain) · **STALE** (old-scheme name, role drifted)
· **UNCHECKED** (evidence insufficient) · **CONFLICT** (self-contradictory / duplicate symbol).

---

## 0. Tally

| Outcome | Count |
|---|---|
| RENAMED (applied + saved to Ghidra) | **18** |
| VERIFIED correct, kept as-is | 7 |
| MISLEADING — proposal recorded, NOT applied (residual doubt) | 4 |
| CONFLICT — spurious duplicate symbol, needs **deletion** (out of read-only scope) | 2 |
| Duplicate candidates (same addr surfaced in 2 categories) | 3 |

`save_program` returned success; all 18 unique renames read back confirmed before save. Convention
warnings (underscores / non-PascalCase) were advisory only and match the project's `Class__Method`
convention.

> **Slice 2 (2026-05-30):** +49 renames, +2 deletions applied & saved (see **§9**); 2 skipped pending
> `create_function`. All §3 proposal-only labels and §4 spurious symbols below were resolved in Slice 2.
>
> **Slice 3 (2026-05-30):** +149 operations applied & saved (see **§11**) — exhaustive IPersist
> (GetClassID/Load/Save) + scalar-deleting-destructor + dtor sweep across ~50 AbstractClass-derived
> class vtables; the 2 slice-2 ticks cleared via `create_function`. 2 over-proposals correctly skipped;
> 2 sweep groups failed and need a re-run (g7-anim-effects, g10-mission-scripting).
>
> **Cumulative across all three slices: ~216 renames/creates + 2 deletions committed to Ghidra.**

---

## 1. RENAMED — applied to Ghidra this pass (18)

Each was re-verified independently by the writer (current label confirmed → evidence re-read →
rename → read-back). Address-first verified names.

| # | Address | Old label | New label | Why (verified) |
|---|---|---|---|---|
| 1 | `0x004CA230` | `FactoryClass__Update` | `FactoryClass__GetClassID` | IPersist slot 3 (vtable `0x007e88d0`+0xC). Copies 16-byte GUID `a8d9ec34-b00a-d211-aca7-006008055bb5` from `0x007e9820`, returns HRESULT, `RET 8`. Not a tick. |
| 2 | `0x0065AEB0` | `MissionClass__Constructor` | `RadioClass__ScalarDeletingDestructor` | RadioClass vtable `0x007f0508` **slot 8** (+0x20). Sets vtable at top, frees `+0xe4`, down-resets to MissionClass vtable, chains `ObjectClass__Destructor`, bit0 conditional free, `RET 4`. **Double-wrong** (dtor not ctor; RadioClass not MissionClass). |
| 3 | `0x005B3A60` | `MissionClass__Constructor` | `MissionClass__ScalarDeletingDestructor` | MissionClass vtable `0x007edcc0` **slot 8** (+0x20). Vtable reset → chain `ObjectClass__Destructor` → bit0 free → `RET 4`. Collided with the real ctor `0x005b2da0`. |
| 4 | `0x004CA770` | `FactoryClass__vtable_8` | `FactoryClass__ScalarDeletingDestructor` | Primary vtable `0x007e88d0` slot 8 (`0x007e88f0`). DEC `g_FactoryClass_Array_Count`+shift, `AbandonProduction` gated by `g_GameActive`, chains `AbstractClass__Destructor_ResetVtables`, bit0 free. |
| 5 | `0x004CA270` | `FactoryClass__vtable_5` | `FactoryClass__Load` | IPersist slot 5 (`0x007e88e4`). Calls `AbstractClass__Load`, rebuilds QueuedObjects + vtable chain, IStream::Read. Symmetric with slot 6 Save. |
| 6 | `0x004CA3C0` | `FactoryClass__vtable_6` | `FactoryClass__Save` | IPersist slot 6 (`0x007e88e8`). Calls `AbstractClass__Save`, IStream::Write of `+0x50` + `+0x44` element array, `RET 0xc`. |
| 7 | `0x004d3590` | `FootClass__Constructor` | `FootClass__Destructor` | FootClass vtable `0x007e8c94` slot 8 wrapper is `0x004e0170`. Releases locomotor `+0x19d`, cell-occupancy unlink, SoundEvent/Voc detach, chains `MissionClass__Destructor 0x006f4500`. **Called from the Aircraft/Infantry/Unit dtor chains** — highest poison. |
| 8 | `0x00414080` | `AircraftClass__Constructor` | `AircraftClass__Destructor` | Vtable at top, no AssignUniqueID/CoCreate. DEC `g_AircraftClass_Array_Count`+shift, removes from tagged heap `DAT_00b0e840`, tail-chains FootClass dtor. Scalar-del wrapper `0x0041c210`. Genuine ctor is `0x00413d20`. |
| 9 | `0x00517d90` | `InfantryClass__Constructor` | `InfantryClass__Destructor` | Vtable `0x007eb058` at top; SlaveManager detach, `FootClass__Limbo`, DEC `g_InfantryClass_Array_Count`+shift, chains FootClass dtor. Slot 3 wrapper `0x00523350`. Genuine ctor `0x00517a50`. |
| 10 | `0x00735780` | `UnitClass__Constructor` | `UnitClass__Destructor` | Vtable `0x007f5c70` at top; `Remove_Tracking`, `FootClass__Limbo`, rally-point cleanup, DEC `g_UnitClass_Array_Count`+shift, chains FootClass dtor. Sole caller `UnitClass__ScalarDelDestructor 0x00746e80`. Genuine ctor `0x007353c0`. |
| 11 | `0x0055f1e0` | `House_AI_Tick` | `RenderDebugStatsOverlay` | Formats UTF-16 `Frame: %d`/`FPS: %d`/`Resp Time` HUD strings, blits via display-chain vtable slots 0xc/0x10/0x78. Gated by debug flag `DAT_00a8b8b5`. **No house iteration / AI / RNG.** `active_in_yr=no` (dev-gated). |
| 12 | `0x004f83c0` | `HouseClass__AI_Tick` | `HouseClass__Find_Factory` | Searches `g_FactoryClass_Array` for the factory owned by `param_1` whose produced-object RTTI (`vtable+0x84`) == `param_2`. Pure lookup, **no timers/scheduling**. |
| 13 | `0x005f5f30` | `ObjectClass__GetHeight` | `ObjectClass__GetCoordZ` | `MOV EAX,[ECX+0xa4]; RET` — raw absolute Z lepton, **no terrain subtraction**. The real height-above-ground GetHeight is the adjacent `0x005f5f40` (subtracts ground + bridge offset). Was a duplicate-label collision. |
| 14 | `0x0046be10` | `BulletTypeClass__ReadINI_wrapper` | `BulletTypeClass__Destructor` | Stores 4 BulletTypeClass vtables, `Detach_From_All_Lists`, two array-removal loops (DEC `g_BulletTypeClass_Array_Count`+shift), chains ObjectTypeClass dtor `0x005f7400`. **No CCINI reads.** Real reader is `0x0046bee0`. |
| 15 | `0x0045fe50` | `BuildingTypeClass_ReadINI_Water` | `BuildingTypeClass__ReadINI` | Full `[BuildingType]` reader: chains `TechnoTypeClass__ReadINI 0x00712170`, parses BuildCat/AntiAir/AntiArmor/AntiInfantry/MaxNumberOccupants/HasSpotlight/… (none water-specific). Vtable `0x007e45c0` slot 5. The `_Water` suffix was wrong. |
| 16 | `0x005b3760` | `MissionClass__Read_INI` | `MissionControlClass__Read_INI` | Reads `[<MissionName>]` section (`g_MissionNameTable`) NoThreat/Zombie/Recruitable/Paralyzed/Retaliate/Scatter/Rate/AARate into a per-mission config struct. Receiver = the 32-slot stride-`0x20` table at `0x00a8e3a8`, **not** a per-unit MissionClass. Sole caller `RulesClass__ReadTypeData`. |
| 17 | `0x00484ab0` | `CellClass__IsLowBridgeCell` | `CellClass__IsTubeCell` | Tests `cell+0x116` against tube count `DAT_008b4148` + LandType==10; `GetTubeAtCell 0x00484f20` indexes `g_TubeArray` (RTTI `DynamicVectorClass<TubeClass*>`) on the same field. **TS-legacy tunnel/subterranean** — wrong-system "bridge" label. `active_in_yr=no`. |
| 18 | `0x0056d430` | `MapClass__CellCoordToLinearIndex` | `MapClass__CoordToZoneLinearIndex` | Index = `(recv+0xf8 + 1 + recv+0xf4)*y + x` (packed playfield stride), **not** the `y*0x200+x` cell-table frame. All callers feed the zone array `+0x70` (clamp vs `+0x6c`), never the `+0x13c` cell table. The exact coordinate-frame bug class. |

---

## 2. VERIFIED — label correct, kept (7)

| Address | Label | Verified role / note |
|---|---|---|
| `0x004f8440` | `HouseClass__Update` | HouseClass primary vtable slot +0x5C (idx 23). Genuine per-house economy+AI tick (RecheckPower/Radar, PowerOutput/Drain floor, Scatter, MPlayer_Defeated, AI build queue). Slightly understates scope but `Update` is the correct virtual-tick convention. (Detailed AI-vs-player gating captured in the verdict notes — useful for the Rust port.) |
| `0x0055afb0` | `LogicClassPerTickUpdateLiveVector` | The master per-tick logic scheduler; sole caller `Main_Tick 0x0055d360`. **Ordering correction:** cell-action sweep → lightning/weather/storm → Tiberium growth+spread → BombClass::UpdateAll → TeamClass(filtered) → DiskLaser → EMPulse-array `DAT_00b04bd4` → main object live-vector → `DAT_00a83e04` (gamemode-gated) → g_Tactical → Factory → House. Every per-object dispatch is `CALL [vtable+0x5C]`. Lockstep-critical — Rust must preserve this exact order. |
| `0x0055d360` | `Main_Tick` | The live frame driver (loop in `Main_Game 0x0048ccc0`). **Correction:** the `Map__Logic 0x004d2370` call here only flags cells dirty; the heavy per-object logic runs later via `LogicClassPerTickUpdateLiveVector`. State-hash record/verify + `Desync_Handler 0x0048dc90` confirm lockstep; `g_CurrentFrameCounter++` at end-of-frame is the determinism boundary. |
| `0x004104c0` | `AbstractClass__GetCoords` | Base-class default returning fixed `{0,0,0}` from the all-zero CoordStruct at `0x00887680`. Owns **no** AbstractClass offset; canonical per-instance triple is ObjectClass+0x9c. (Opportunity: label `0x00887680` `g_ZeroCoord`.) |
| `0x004ca130` | `FactoryClass__IsComplete` | `(Object[+0x58]!=0 OR SpecialItem[+0x68]!=-1) AND Progress[+0x24]==0x36`. Confirms +0x24=Production_Value (ceiling 0x36=54), +0x58=Object, +0x68=SpecialItem (-1 sentinel). |
| `0x004ca120` | `FactoryClass__GetProgress` | `MOV EAX,[ECX+0x24]; RET`. **+0x24 is a raw 0..54 step counter, NOT a percentage** — binary divides by 0x36 for the cameo build-clock. Any "progress %" must scale by /54. |
| `0x005f65a0` | `ObjectClass__GetCoords` | Canonical coord anchor: copies +0x9c(X)/+0xa0(Y)/+0xa4(Z) leptons. Fixed inherited virtual slot 0. **This is the reference for the offset frame** (cell = lepton>>8, sign-corrected). |
| `0x00486770` | `CellClass__IsWoodBridge` | Tests IsoTile index (+0x38) in the `WoodBridgeSet` range `DAT_00abad1c`. The flag's premise (that `DAT_00aa0738` is the low-bridge range) was the error: `aa0e28`=BridgeSet(high), `abad1c`=WoodBridgeSet(low), `aa0738`=WaterSet. |
| `0x0056da10` | `MapClass__FindBridgeRecord` | Locates the high-kind BridgeRecord spanning a cell (filters `+0xC`==0 = bridge_kind high). Omitting the `+0x08` is_intact check is **by design** — callers (`GetZoneID`, `ResolvePathCoord_BridgeAware`) read is_intact themselves. |

*(7 distinct VERIFIED labels above; two of the FactoryClass entries from §1 — Load/Save — were
"VERIFIED role wearing a generic `vtable_N` placeholder", so they appear under RENAMED.)*

---

## 3. MISLEADING — verified wrong, proposal recorded only (NOT applied) — ✅ all 4 RESOLVED in Slice 2 (§9)

Held back from auto-rename because of residual doubt on the *replacement* name (the current label is
nonetheless confirmed wrong — do not trust it).

| Address | Current label | Verdict | Proposed (unconfirmed) | Why held |
|---|---|---|---|---|
| `0x006f32d0` | `BuildingTypeClass__ReadINI` | **MISLEADING** | `TechnoClass__IsControlledByLocalHumanPlayer` | Definitely NOT an INI reader: a shared TechnoClass/FootClass virtual (slot +0x64, idx 25) bool predicate ending in `HouseClass__IsHumanPlayer` on owner `[this+0x21c]`. But upstream gates (`+0x81`,`+0x3d5`,`+0x41b`, type-enum!=6, `[type+0x230]`) aren't fully named → exact predicate semantics unpinned. The real BuildingType reader is `0x0045fe50` (renamed in §1). |
| `0x00565730` | `CellClass__Get_Cell_At` | **MISLEADING** | `MapClass__Coord_To_CellClass` | Input is a **CoordStruct (leptons), ÷256**, not a cell coord (`SAR 8` then `y*0x200+x`). Load-bearing correction is certain; only the class prefix (Map vs Cell) carries doubt. Hundreds of live coord-space callers. |
| `0x00485060` | `CellClass__IsOnBridgeSurface` | **MISLEADING** | `CellClass__IsBridgeDeckTile` | Tests **only** the 14-type wood-deck range `DAT_00aa0738`; excludes ramps (`IsOnBridgeRamp 0x00578d80`) and directional edge ranges. "OnBridgeSurface" overpromises. Held: original is directionally correct, deck-vs-surface is a naming nuance. |
| `0x00410600` | `ObjectClass__GetCoords` | **MISLEADING** | (strip label; thunk) | A `+4` this-adjustor thunk (`SUB [ESP+4],4; JMP 0x00410310`) that owns no coord offset. Recommend **stripping** the GetCoords label regardless; finalize a role name only after verifying the target stub `0x00410310` (a `MOV EAX,1; RET 4` currently labeled `AbstractClass__Release`, slot/callers unverified). Coord semantics stay anchored on `0x005f65a0`. |

---

## 4. CONFLICT — spurious duplicate symbol, needs DELETION — ✅ both DELETED in Slice 2 (§9)

These are not renames — Ghidra carved a **second function start mid-instruction** over a real
function. Deletion is a write op outside this read-only verify pass; queued for the next slice.

| Bogus symbol | Real owner | Action |
|---|---|---|
| `0x005fe7a0` `OverlayTypeClass__ReadINI` | `0x005fe770` (the real `OverlayTypeClass__ReadINI`, sole vtable xref `0x007ef664`) | `delete_function 0x005fe7a0` — it is the interior `LEA EDI,[ESI+0x24]` of `0x005fe770`. |
| `0x00428319` `BulletTypeClass__ReadINI_Part2` | `0x00427d00` (`AnimTypeClass__ReadINI`) | `delete_function 0x00428319` — mid-body fragment of the Anim reader (reads AnimType keys, no prologue). The real `BulletTypeClass__ReadINI` is `0x0046bee0`, a separate complete function. |

---

## 5. Top bad labels (ranked by poison risk)

1. **`FootClass__Constructor` @ `0x004d3590` → was the shared FootClass DESTRUCTOR.** Worst offender:
   it is called from the Aircraft/Infantry/Unit destructor chains, so anyone reading "the FootClass
   constructor" would model **unit spawn from teardown code**. Now `FootClass__Destructor`.
2. **The dtor-labeled-as-ctor family** (`Aircraft/Infantry/Unit/Mission/Radio` `__Constructor`).
   Each inverted lifecycle reasoning *and* created a duplicate-label collision with the genuine
   constructor. RadioClass's was double-wrong (also the wrong class). All 5 fixed.
3. **`House_AI_Tick` / `HouseClass__AI_Tick`** — neither is a scheduler. One is a debug HUD renderer
   (dev-gated, not in YR), the other a factory-lookup helper. "AI_Tick" would inject non-tick code
   into the lockstep tick pipeline. Both fixed.
4. **`MapClass__CellCoordToLinearIndex` @ `0x0056d430`** — the exact coordinate-frame bug class from
   CLAUDE.md: a zone-index frame (packed stride) labeled as the cell-index frame (`*0x200`). Porting
   it onto the cell table would silently corrupt every cell lookup. Now `…CoordToZoneLinearIndex`.
5. **`BuildingTypeClass_ReadINI_Water` vs `BuildingTypeClass__ReadINI`@`0x006f32d0`** — the real
   `[BuildingType]` reader wore a "_Water" suffix while a non-reader predicate held the canonical
   `ReadINI` name. Misroutes any INI-parsing research. Real reader relabeled; the impostor flagged.
6. **`FactoryClass__Update` @ `0x004CA230` → `GetClassID`** (the seed). Confirmed the systemic IPersist
   slot-3 GUID-copy pattern; the real Factory tick is `FactoryClass__AI 0x004C9B20` (slot +0x5C).

---

## 6. Cross-cutting patterns confirmed

- **IPersist/COM vtable layout** (every AbstractClass-derived class): slot 0 QueryInterface, slot 3
  **GetClassID** (16-byte GUID copy, HRESULT, `RET 8`), slot 5 **Load**, slot 6 **Save**, slot 8
  **scalar-deleting destructor** (tests stack flag bit0 → optional free). Slot-3 "*__Update" and
  slot-8 "*__vtable_8" labels are the recurring mislabel signatures.
- **VC++6 dtor-as-ctor**: a function that sets the leaf vtable at the top but then DECREMENTS a
  per-class array + shifts down + chains a parent `__Destructor`/`…ResetVtables` is a destructor.
  Genuine constructors chain the *base ctor first*, `AssignUniqueID`, `CoCreateInstance` locomotor,
  and INCREMENT the array.
- **Per-object tick virtual = vtable slot +0x5C (idx 23)** across all live classes dispatched by
  `LogicClassPerTickUpdateLiveVector`. (Slot bytes per-class not byte-verified this pass — the live
  arrays are empty under static analysis; see next slice.)
- This pass **confirms the round-39 `LABEL_AUDIT_LOG.md` hypothesis**: the `MissionClass__Constructor`
  labels reached from destructor chains are destructors (here `0x0065AEB0`=RadioClass scalar-del dtor,
  `0x005B3A60`=MissionClass scalar-del dtor).

---

## 7. Unresolved conflicts (carry-forward)

- `0x005fe7a0`, `0x00428319` — spurious mid-body function symbols; need `delete_function` (§4).
- `0x006f32d0` — confirmed NOT `BuildingTypeClass__ReadINI`; exact predicate semantics (slot +0x64
  family) unpinned before renaming.
- `0x00410600` — GetCoords label is wrong (this-adjustor thunk); strip it, then name the target stub
  `0x00410310` only after verifying its slot/callers (Release vs no-op vs predicate).
- `0x004f83c0` (`HouseClass__Find_Factory`) — renamed, but `param_2`'s RTTI-type role is body-inferred
  only (zero recorded callers); confirm a caller before treating as live. `active_in_yr` unknown.

---

## 8. Next audit slice

1. **Scalar-deleting-destructor wrappers** surfaced but unnamed: `0x004e0170` (Foot), `0x0041c210`
   (Aircraft), `0x00523350` (Infantry) → `*__ScalarDeletingDestructor`. (`0x00746e80` already named.)
2. **Delete the two spurious symbols** (`0x005fe7a0`, `0x00428319`) and re-decompile the owners.
3. **IPersist slot-3 GetClassID sweep** across the other class vtables (Unit/Infantry/Building/House/
   Cell) — same GUID-copy mislabel as FactoryClass likely recurs.
4. **vtable slot +0x5C (idx 23) per-class targets** — byte-verify each live class's tick virtual
   against its label (Factory__AI, House__Update, Team/DiskLaser/EMPulse). Requires a loaded savegame
   or live arrays, since static arrays are empty.
5. **Pin `0x006f32d0` predicate semantics** (the TechnoClass/FootClass slot +0x64 family).
6. **Globals worth labeling** that surfaced: `g_MissionControl_Array @ 0x00a8e3a8`,
   `g_ZeroCoord @ 0x00887680`, tile-set bases `g_WaterSet @ 0x00aa0738` /
   `g_BridgeSet @ 0x00aa0e28` / `g_WoodBridgeSet @ 0x00abad1c`, `g_TubeCount @ 0x008b4148`.
7. **Third ObjectClass height-region slot `0x005f5fa0`** (between GetHeight and GetCoordZ) — unexamined.

---

## 9. Slice 2 — 2026-05-30 (applied + saved)

Workflow `wf_eff87ade-30e` (15 agents, 842 Ghidra calls). Same verify→serialized-write discipline,
5 sequential writer groups (each re-verified from a fresh binary read; `save_program` succeeded per
group). **49 renames + 2 deletions committed; 2 candidates correctly skipped.**

### 9a. Tier-1 — pre-specified, re-verified + applied (9 renames + 2 deletions)

| Address | Old | New | Note |
|---|---|---|---|
| `0x004e0170` | `FUN_004e0170` | `FootClass__ScalarDeletingDestructor` | calls FootClass dtor then bit0 free, RET 4 |
| `0x0041c210` | `FUN_0041c210` | `AircraftClass__ScalarDeletingDestructor` | calls Aircraft dtor `0x00414080` then bit0 free |
| `0x00523350` | `FUN_00523350` | `InfantryClass__ScalarDeletingDestructor` | calls Infantry dtor `0x00517d90` then bit0 free |
| `0x00a8e3a8` | `DAT_00a8e3a8` | `g_MissionControl_Array` | 32-slot stride-0x20 table; consumer `RulesClass__ReadTypeData` |
| `0x00887680` | `DAT_00887680` | `g_ZeroCoord` | all-zero CoordStruct read by `AbstractClass__GetCoords` |
| `0x00aa0738` | `DAT_00aa0738` | `g_WaterSet_TileSetBase` | `General/WaterSet` INI base (wood-bridge deck tiles live at start of this range) |
| `0x00aa0e28` | `DAT_00aa0e28` | `g_BridgeSet_TileSetBase` | `General/BridgeSet` (high concrete bridge) |
| `0x00abad1c` | `DAT_00abad1c` | `g_WoodBridgeSet_TileSetBase` | `General/WoodBridgeSet` (low/wood bridge) |
| `0x008b4148` | `DAT_008b4148` | `g_TubeCount` | tube count (paired `g_TubeArray @ 0x008b413c`); TS-legacy |
| `0x005fe7a0` | `OverlayTypeClass__ReadINI` | **DELETED** | spurious mid-instruction start inside `0x005fe770` (real reader); 0 xrefs |
| `0x00428319` | `BulletTypeClass__ReadINI_Part2` | **DELETED** | spurious fragment inside `AnimTypeClass__ReadINI 0x00427d00`; 0 xrefs |

### 9b. Tier-2 — held-back names pinned + applied (4)

| Address | Old | New | Pinned by |
|---|---|---|---|
| `0x006f32d0` | `BuildingTypeClass__ReadINI` | `TechnoClass__IsLocalPlayerSelectableObject` | slot +0x64 (idx 25) bool predicate; gates alive + non-building + `Selectable=` (Type+0x230) + `IsHumanPlayer` owner; caller `0x004aa2b0` is the on-screen next/prev selection cursor scan |
| `0x00565730` | `CellClass__Get_Cell_At` | `MapClass__Get_CellClass_At_Coord` | receiver is MapClass (owns +0x13c table per ctor `0x00565090`); lepton ÷256 input, distinct from cell-coord sibling `0x005657a0` |
| `0x00485060` | `CellClass__IsOnBridgeSurface` | `CellClass__IsBridgeDeckTile` | tests ONLY the 14-tile wood-deck range; excludes ramps/edges (`HasBridgeOverlay 0x004865d0` is the broad one) |
| `0x00410600` | `ObjectClass__GetCoords` | `AbstractClass__Release_secondary4_adjustor_thunk` | `SUB [ESP+4],4; JMP 0x00410310` (=`AbstractClass__Release`); secondary-4 vtable adjustor, owns no coord |

### 9c. Tier-3 sweeps — new mislabels found + applied (36)

**IPersist GetClassID (slot 3, +0x0C) — generic `FUN_` → `Class__GetClassID` (6):**
`0x0071D310` Terrain · `0x0041C190` Aircraft · `0x0062D930` Particle · `0x00721E40` Tiberium ·
`0x0065B470` RadSite · `0x0074AAD0` VoxelAnim. (Each: E_POINTER on null, 16-byte GUID copy, RET 8.)

**IPersist Load/Save (slots 5/6) — generic `FUN_` → `Class__Load`/`Class__Save` (8):**
`0x005f5e80` `ObjectClass__Load` · `0x0065ab80`/`0x0065ac40` `RadioClass__Load`/`Save` ·
`0x0070bf50`/`0x0070c250` `TechnoClass__Load`/`Save` · `0x005f9720`/`0x005f9950` `ObjectTypeClass__Load`/`Save` ·
`0x007162f0`/`0x00716dc0` `TechnoTypeClass__Load`/`Save`. (Each chains the parent `AbstractClass`/base `__Load`/`__Save`.)

**Destructors mislabeled as `__Constructor` / generic → `Class__Destructor` + `Class__ScalarDeletingDestructor` (slot 8) (20):**

| Class | Destructor (was) | ScalarDeletingDestructor (was) |
|---|---|---|
| CellClass | `0x0047bb60` (`CellClass__Constructor`) | `0x00487e80` (`FUN_`) |
| SlaveManagerClass | `0x006af4a0` (`__Constructor`) | `0x006b1390` (`FUN_`) |
| RadSiteClass | `0x0065b2f0` (`__Constructor`) | `0x0065bed0` (`FUN_`) |
| SuperClass | `0x006cb120` (`__Constructor`) | `0x006ce220` (`FUN_`) |
| **HouseClass** | `0x004f7140` (**`BaseClass__Constructor`** — doubly-wrong) | `0x0050e380` (`FUN_`) |
| TagClass | `0x006e4f60` (`__Constructor`) | `0x006e58b0` (`FUN_`) |
| TeamClass | `0x006e8de0` (`__Constructor`) | `0x006f0450` (`FUN_`) |
| TerrainClass | `0x0071b7b0` (`__Constructor`) | `0x0071d350` (`FUN_`) |
| BulletClass | `0x00466560` (`__Constructor`) | — (wrapper `0x0046b5c0` left) |
| TiberiumClass | `0x00721880` (`__Constructor`) | — (wrapper `0x00723710` left) |
| AbstractTypeClass | `0x004109c0` (`__Constructor`) | — |
| Tactical | — | `0x006dc470` (`Tactical__Constructor` → scalar-del dtor) |

**Other duplicate-label collision resolved (1):** `0x00587410` `CellClass__GetRadarColor` →
`MapClass__FindBridgeConnection_Predicate` (scans a 5×5 neighborhood of bridge tilesets for an
`InfantryClass__What_Action` predicate — no radar work). The **real** `GetRadarColor` is the
duplicate-named `0x0047c060` (radar callers), left intact.

### 9d. Skipped — carry-forward (need `create_function` first)

The `+0x5C` tick-virtual sweep confirmed two slot-23 targets by vtable bytes + prologue, but Ghidra
has **no defined function object** at the address (undefined region), and the writer's toolset lacked
`create_function` — so it refused to rename rather than fabricate success:

- `0x006e9140` → `TeamClass__AI` (vtable__TeamClass +0x5C = `40916e00`; prologue confirmed).
- `0x00760f50` → `WaveClass__AI` (vtable__WaveClass +0x5C = `500f7600`; prologue confirmed).

**Next:** `create_function` at each address, then apply the (already-verified) rename.

### 9e. Notable

- **Doubly-wrong label caught:** `0x004f7140` was `BaseClass__Constructor` — actually the
  `HouseClass` **destructor** (installs HouseClass vtables, decrements 5 listener lists, resets the
  embedded BaseClass sub-object). Same class-and-role error pattern as the slice-1 RadioClass case.
- The slot-8 scalar-deleting-destructor + the `__Constructor`-is-really-a-destructor pattern is now
  confirmed across **~15 classes** — strong evidence the old labeling script systematically mislabeled
  destructors as constructors. Remaining classes should be swept the same way.

### 9f. Next audit slice (updated)

1. `create_function` + rename the 2 skipped `+0x5C` ticks (`0x006e9140` TeamClass__AI, `0x00760f50` WaveClass__AI).
2. Continue the dtor-as-ctor sweep across remaining classes (Overlay, Aircraft/Building/Unit *Type*
   variants, the rest of the mission/scripting + light/particle families).
3. Continue Load/Save slot 5/6 + GetClassID slot 3 across the classes not yet swept.
4. Pin `0x006f32d0`'s sibling gates if a finer name is ever needed (current name is accurate).
5. Byte-verify the rest of the `+0x5C` tick virtuals per class (needs a loaded savegame for live arrays).
6. Still open from slice 1: third ObjectClass height-region slot `0x005f5fa0`.

---

## 11. Slice 3 — 2026-05-30 (applied + saved): exhaustive IPersist + dtor sweep

Workflow `wf_9a23c930-109` (25 agents, 1813 Ghidra calls). Swept the IPersist/lifecycle vtable slots of
~50 AbstractClass-derived classes via 12 group-agents → serialized writers. **149 operations applied +
saved** (renames + `create_function`-then-rename), **2 correctly skipped**, all 13 writer groups saved.
Per-item evidence (vtable-slot decode + body confirmation + read-back) for all 149 ops lives in the
run's output; this section summarizes by pattern.

### 11a. Applied, by pattern
Each op re-verified by the writer from the owning class's vtable slot bytes + the function body, then read
back:
- **GetClassID** (slot 3, `+0x0C`): per-class CLSID accessors, mostly undefined or generic — e.g.
  `InfantryClass__GetClassID` (`0x00523300`).
- **Load / Save** (slots 5/6, `+0x14`/`+0x18`): per-class IPersistStream serializers — the bulk of the
  haul. Includes genuine *mislabels* (not just generic): `UnitClass__Load` was **`UnitClass__Draw_It`**,
  `UnitClass__Save` was **`UnitClass__UpdatePosition`**; plus `InfantryClass__Load`/`Save`,
  `AircraftClass__Save`, `TacticalClass__Load`/`Save`, etc. Most were `create_and_rename` (the serializer
  thunks weren't even defined as functions in Ghidra — only a vtable DATA slot pointed at them).
- **ScalarDeletingDestructor** (slot 8, `+0x20`) + the full **Destructor** it calls: e.g.
  `ObjectClass__ScalarDeletingDestructor` (`0x005f6dc0`), `AbstractTypeClass__ScalarDeletingDestructor`,
  and `FoggedObjectClass__ScalarDeletingDestructor` + `FoggedObjectClass__Destructor` (was
  `__Constructor`) — the dtor-as-ctor pattern recurring on the remaining classes.
- **AI/Update tick** (slot 23, `+0x5C`): the 2 slice-2 carryovers `TeamClass__AI` (`0x006e9140`) and
  `WaveClass__AI` (`0x00760f50`) — both `create_function`'d then renamed — plus `TacticalClass__AI`
  (`0x006d2540`).

### 11b. Discipline note — 2 over-proposals skipped
The serialized writer refused 2 sweep proposals whose fresh read contradicted them:
`ScriptTypeClass__AI` (`0x006918a0`) and `TaskForceClass__AI` (`0x006e8420`) were proposed as slot-23
ticks but are actually **slot 25 (`+0x64`)** INI/load-style methods — the same `+0x64` family as the
slice-2 `TechnoClass__IsLocalPlayerSelectableObject`. Left unrenamed; flagged for a proper look.

### 11c. Gaps — 2 sweep groups failed (re-run = slice 3b)
Two group-agents returned no structured output (harness retries exhausted), so their classes were **not
swept**:
- **g7-anim-effects:** AnimClass, VoxelAnimClass, ParticleClass, ParticleSystemClass, AlphaShapeClass,
  BuildingLightClass.
- **g10-mission-scripting:** TagClass, TagTypeClass, TriggerClass, TriggerTypeClass, TActionClass,
  TEventClass.

### 11d. Intentionally deferred (project policy)
- **AI:** NeuronClass, AITriggerTypeClass, BrainClass (`feedback_no_ai_yet`).
- **TS-legacy:** TubeClass, VeinholeMonsterClass (`feedback_no_tunnel_subterranean`).
- Non-game-object vtables (UI/command, file/pipe/straw, network, WDT, score, multiplayer-mode, locomotion
  COM interfaces) — outside the AbstractClass IPersist layer.

### 11e. Layer status
With **g7 + g10 re-run** and the **2 `+0x64` skips** resolved, the AbstractClass-derived
GetClassID / Load / Save / ScalarDeletingDestructor / Destructor / tick layer will be **complete except the
policy-excluded classes**. Until then it is *near-complete* — honest status, not "done."

### 11f. Next
1. **Slice 3b:** re-run g7-anim-effects + g10-mission-scripting (same per-class sweep).
2. Investigate the 2 slot-25 (`+0x64`) functions (`0x006918a0`, `0x006e8420`) — likely the
   selectable/local-control predicate family.
3. (carryover) third ObjectClass height-region slot `0x005f5fa0`.

---

## Sources

Live Ghidra MCP decompilation/disassembly/`read_memory`/`get_function_callers`/`get_xrefs_to` against
gamemd.exe (43-agent workflow, 861 Ghidra calls). Per-symbol verification calls are recorded in each
verdict's `ghidra_calls` (workflow run `wf_5526cd68-d99`). Renames applied + `save_program` confirmed.
Extends `LABEL_AUDIT_LOG.md`; cross-refs `GHIDRA_LABEL_SPOTCHECK_ABSTRACT_DTOR_004101F0.md`,
`GHIDRA_LABEL_SPOTCHECK_FACTORYCLASS_VTABLE_RANDOM.md`.
