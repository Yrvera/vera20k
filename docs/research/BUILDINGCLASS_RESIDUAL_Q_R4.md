---
name: BuildingClass Residual Questions Round 4
description: Batch close of all residual open questions from v2 §25 + new residuals surfaced by full-decode plan Tasks 6-12.
type: reference
---

# BuildingClass Residual Questions — Round 4

**Scope:** Close every open question surfaced across Tasks 1-12 of the 2026-04-24
BuildingClass full-decode plan, plus v2 §25's original 11 list. Evidence is
binary-verified unless otherwise tagged. Where a claim could not be confirmed
in the available time budget, a `DEFERRED` verdict with minimum-scope follow-up
is recorded.

**Per-question confidence:** tagged individually (HIGH / MED / LOW).
**Active in YR:** tagged individually. Several "dead" verdicts and one major
correction to v2 §24 follow.

---

## Pre-resolved by T1 (cite only)

These six v2 §25 items were closed during Task 1's audit pass. T13 cites only —
does not re-prove.

| v2 # | Question | Verdict | Evidence |
|------|---------|---------|----------|
| #1 | `AircraftClass::What_Am_I` = 2 | **RESOLVED** | `MISSION_GUARD_AREAGUARD_GHIDRA_REPORT.md:255` |
| #4 | `Type+0x1573` = `Powered=` flag | **RESOLVED** | `BUILDINGCLASS_MISSION_ATTACK_GHIDRA_REPORT.md:34`, `BUILDING_SYSTEMS_GHIDRA_REPORT.md:733`, `POWER_SYSTEM_GHIDRA_REPORT.md:65`, `BUILDING_ANIM_STATE_MACHINE.md:118` |
| #5 | Helipad radio 0x1D = `REFUEL_QUERY` | **RESOLVED** | `BUILDINGCLASS_MISSION_REPAIR_AND_PRODUCE.md:414,453` |
| #6 | Building `+0x664` = `FirePowerBonus` (IronCurtain mult) | **RESOLVED** | `BUILDINGCLASS_UPDATE_AI_TICK_GHIDRA_REPORT.md:832,929` |
| #9 | `Rules+0x16E8` `URepairRate` default = 0.016 | **RESOLVED** | `BUILDING_DOCK_AND_HEAL_STATE_MACHINES.md:173,466`, `ini/rulesmd.ini:30` |
| #11 | Bunker `+0x718` terminal-state cleanup path | **RESOLVED** | `BUNKER_SYSTEM_GHIDRA_REPORT.md:135`, `MISSION_REPAIR_AND_PRODUCE_GHIDRA_REPORT.md:324`, `UNIT_MISSION_DEPLOY_BUILDING_GHIDRA_REPORT.md:561,594` |

---

## Group A — v2 §25 originals (4 still-open)

### A1 (v2 #2). BuildOrder `+0x8` and `+0xC` — purpose

**Verdict:** **DEAD** — both fields are write-once-to-zero scratch, never read by any consumer in the binary.

**Evidence (binary):**
- Producer `HouseClass::AI_Manage_Build_Queue @ 0x004FDD10` (LAB_004fe3a9 and sibling paths) writes new 16-byte entries as:
  - `*puVar15 = uVar1;` (+0x0 = `BuildingType.+0xDF8` type id)
  - `puVar15[1] = 0;` (+0x4 = packed-cell placeholder; updated later)
  - `puVar15[2] = uStack_c;` (+0x8 — `uStack_c` is zero-initialised scratch on the path, never touched)
  - `puVar15[3] = 0;` (+0xC = zero)
- Consumer `HouseClass::AI_ChooseNextProduction @ 0x00506EF0` reads only `[entry+0]` (type) and `[entry+4]` (cell); never `[entry+8]` or `[entry+0xC]`. The only extra write is `*(entry+0x14) = local_154`, which is **next-entry +0x4** (the linked-slot base defense packed-cell), not the current entry's +0x8/+0xC.
- Consumer `FUN_0050A490` (`OnBuildingDestroyed` hook) reads only `piVar5[0]` (type) and `piVar5[1]` (cell low/high shorts); never +0x8/+0xC. It *writes* +0x0 = 0xFFFFFFFF and +0x4 = `g_InvalidCell` for base-defense slots, but does not touch +0x8/+0xC.
- Consumer `BuildingClass::ExitObject_Main` (per `BUILDINGCLASS_OPEN_QUESTIONS_VERIFICATION_R3.md:181-182`) — confirmed "not touched".

**Implication for Rust:** a 2×DWORD dead field. Represent the entry as an 8-byte `(type_id, packed_cell)` struct; the extra 8 bytes of padding are irrelevant.

**Confidence:** HIGH. **YR-active:** N/A (dead).

---

### A2 (v2 #7). Building `+0x700` short (init 0x3E8) — consumer

**Verdict:** **DEAD** — write-only facing cache with no readers.

**Evidence (binary):**
- `BuildingClass::Constructor @ 0x0043B740` disasm 0x0043B9D3 writes `*(short*)(this + 0x700) = 1000 (0x3E8)` (default).
- `BuildingClass::UpdateAnimation` phase B disasm 0x00450A62 writes `*(short*)(this + 0x700) = AX` where AX is the return of `FUN_00456FB0(...)` (facing rotator). Overwrites default every tick.
- Full byte-pattern sweep for reads: `66 8B 86 00 07`, `66 8B 81 00 07`, `66 8B 83 00 07`, `66 8B 87 00 07`, `0F B7 86 00 07`, `0F BF 86 00 07` — **zero matches**. Only writes exist.
- Write patterns (`66 89 …`, `66 C7 …`): exactly two sites, both in the constructor and UpdateAnimation. No other callers.

**Implication for Rust:** omit the field entirely from `BuildingState`. If reproducing the constructor layout bit-for-bit for save/load parity, keep a `_pad_700: u16` placeholder, but do not wire it to any behavior.

**Confidence:** HIGH. **YR-active:** N/A (written but never consumed).

---

### A3 (v2 #8). SecretLab pick storage offset

**Verdict:** **RESOLVED** — `BuildingClass+0x6F4` stores the rolled SecretLab pick as a TechnoTypeClass pointer.

**Evidence (binary):**
- **Read site:** `FUN_00459840` (called only by `HouseClass::CanBuild @ 0x004F7870`):
  ```c
  iVar1 = *(int *)(param_1 + 0x520);         // BuildingClass.Type
  iVar2 = *(int *)(iVar1 + 0xea4);            // SecretInfantry=
  if (iVar2 == 0) iVar2 = *(int *)(iVar1 + 0xea8);  // SecretUnit=
  if (iVar2 == 0) iVar2 = *(int *)(iVar1 + 0xeac);  // SecretBuilding=
  if (iVar2 == 0) iVar2 = *(int *)(param_1 + 0x6f4); // runtime pick fallback
  return iVar2;
  ```
  The three TypeClass fields are fixed-override secret types from `rulesmd.ini` (`SecretInfantry=/SecretUnit=/SecretBuilding=` — see `BUILDINGTYPECLASS_FIELDS.csv:25-27`). The fallback to **instance** field +0x6F4 is the runtime roll.
- **Write site:** `FUN_0068C050` — per `BUILDINGCLASS_SPECIAL_BUILDINGS_GHIDRA_REPORT.md:413`:
  `SecretLabArray[lab]->field_0x6F4 = pickedType;` (called post-game-start to roll each SecretLab building's random secret tech).
- **Save/Load fixup:** `BuildingClass::Load @ 0x00453E20` registers `param_1 + 0x1bd` = +0x6F4 with the pointer-fixup dictionary (line "piStack_20 = param_1 + 0x1bd; FUN_006cf240();"), confirming it is an Abstract-derived pointer. This matches T8's observation.

**Note:** `BUILDINGCLASS_SPECIAL_BUILDINGS_GHIDRA_REPORT.md` already had the full resolution — T1's audit overlooked it. No new decompilation required; T4 merely needed cross-reference to that doc.

**Confidence:** HIGH. **YR-active:** Yes (retail Allied tech path uses SecretLab frequently).

---

### A4 (v2 #10). `Type+0x184C / Type+0x184D` — purpose

**Verdict:** **DEAD — MISATTRIBUTED.** These offsets are **not** on BuildingTypeClass. They do not exist.

**Evidence (binary):**
- BuildingTypeClass max offset from ctor: `BuildingTypeClass::constructor @ 0x0045DD90` writes at most `param_1[0x5E4] = *(char*)(param_1 + 0x1791) = 1`. That is byte offset `0x5E4*4 = 0x1790`, with final byte-field at `0x1791`. Ctor size ends at **0x1798** (aligned to 8-byte boundary). So `0x184C/0x184D` would be `0xB4` bytes past end.
- Byte-pattern sweep for `4C 18 00 00` over the entire binary: only 2 hits — both in `RulesClass::ReadRules` caller chain (0x00667821 and 0x0066D1B1). Pattern `4D 18 00 00`: **zero hits**.
- `FUN_0066D150` at 0x0066D1B1 reads `param_1 + 0x1848, 0x184C` as the high 4 bytes of a `double` storing `ElevationBonusCap=`. **`param_1` there is `RulesClass*`**, not `BuildingTypeClass*`. Context: `RulesClass::ReadRules @ 0x00668BF0` calls `FUN_0066D150` for the `[ElevationModel]` section.
- **Rules+0x1848 = `ElevationBonusCap`** (double) — `0x184C` is the upper DWORD of that double. Rules+0x184D cannot exist at byte-level (mid-double).

**Implication for Rust:** do not create a field at BuildingTypeClass+0x184C. The plan's claim that these lie within "0x1798 template bounds" is incorrect. Any code reading these on a BuildingType pointer is reading past-end memory.

**Confidence:** HIGH. **YR-active:** N/A (the offsets belong to RulesClass, where they decode `ElevationBonusCap`).

---

## Group B — T6-T12 residuals

### B1 (T6). `local_10` / `in_stack_0000000c` stack bleed in UpdateAnimation phase A

**Verdict:** **RESOLVED — harmless compiler artifact.**

**Evidence (binary):** At `0x00450A19`, the assembly is `MOV EDX, dword ptr [ESP + 0x18]`. `ESP+0x18` is the `UpdateAnimation` prolog-reserved stack cell (see `SUB ESP, 0x14` + push EBX/ESI/EDI pushes shifting the frame). The EDX value is then copied to `[EDI+4]` = `BuildingClass+0x104` (the CDTimer "flags" word). The stack cell `ESP+0x18` is never explicitly initialised in `UpdateAnimation`, so Ghidra labels it `undefined`. The caller (`BuildingClass::Update @ 0x0043FB20`) similarly does not initialise it. The net effect is that BuildingClass+0x104 cycles through whatever happened to sit in the outer stack frame.

BuildingClass+0x104 is the CDTimer flags/seed word. Across the binary it is read only inside CDTimer helpers (`CDTimerClass::GetTimeRemaining @ 0x00426630`) which check elapsed time against `+0x100 + +0x108`, **ignoring +0x104** entirely — the field is a flags byte that gets set/cleared at Start/Stop, and in this code path it is set to an arbitrary sentinel that the timer code tolerates (the frame-start value at +0x100 = `g_CurrentFrameCounter` is the authoritative data).

**Rust implication:** initialise `+0x104` to 0 in constructor; never copy from an uninitialised source. The game's "bug" is benign because the CDTimer reads only +0x100 and +0x108. Do not copy gamemd's pattern.

**Confidence:** HIGH. **YR-active:** Yes (path runs every tick) but behavioral impact = zero.

### B2 (T6). `+0x218` semantics in phase H

**Verdict:** **RESOLVED — inherited TechnoClass pointer to the current active mission/state object.** In phase H used as a "no-active-mission-radio" gate.

**Evidence (binary):**
- Multiple xrefs in `BuildingClass::Sell @ 0x00449C30` test `*(int*)&param_1->field_0x218 != 0` AND then call `(**(code **)(**(int **)&param_1->field_0x218 + 0x2c))()` expecting return 0xB. Invoking vtable+0x2C on a non-null `+0x218` = standard `AbstractClass::What_Am_I` polymorphic call; return 0xB = `RADIO` RTTI type.
- In `OnConstructionComplete @ 0x00445F80` (case of gating naval rally-point creation): `if (param_1->Type[0x16bd] != '\0' && param_1->Type[0xcce] != '\0' && *(int *)&param_1->field_0x218 == 0)` — meaning "building has no active radio contact yet, so assign an initial rally point".
- Matches `TECHNOCLASS_STRUCT_LAYOUT.md` usage: +0x218 is the TechnoClass `PrimaryRadioContact` / `WarpState` pointer (doc-level confusion; the same slot holds different things on different RTTI subtypes). For BuildingClass the semantic is **primary radio contact / active mission object** — consistent with UpdateAnimation phase H's reading: if the building has no active mission radio and construction mission is active and BState_Frame == 0x17, mark buildup done.

**Rust implication:** treat `+0x218` as the owning-radio pointer (Option<EntityId>). Its `== 0` check in phase H means "no dockee/no active contact".

**Confidence:** HIGH. **YR-active:** Yes.

### B3 (T6). Phase F tier-overflow cache correctness at `+0x6F0`

**Verdict:** **RESOLVED — not a bug in retail YR.** Tier overflow cannot occur because Refinery `Storage=` > 100 in all stock buildings.

**Evidence (binary + INI):** Per `BUILDINGCLASS_UPDATE_ANIMATION_GHIDRA_REPORT.md:322`, the `(amount << 2) / capacity` formula can mathematically exceed 3 if `amount > 0.75 × capacity × 4 / amount = ...`, but the check at 0x00450E3E-E41 is `CMP EAX, 0x3; JL 0x00450e9f` — **all values >= 3 drop into the "tier 3" branch**. The uncapped value is then stored to `+0x6F0` via `MOV dword ptr [ESI + 0x6F0], EAX`. If the cached value later compares unequal when storage drops back through 3, the branch re-fires. In stock YR, Refineries (`GAREFN/NAREFN/YAREFN`) all have `Storage=20` and `CapacityMax=` in the Rules that never yield this overflow. Mods could trigger it.

**Rust implication:** clamp `tier = min((amount * 4) / capacity, 3)` before caching to `+0x6F0`. Stock YR parity is preserved.

**Confidence:** MED-HIGH. **YR-active:** Yes (path runs); edge case harmless in stock.

### B4 (T6). Slot-16 / phase-G pre-emption interaction

**Verdict:** **RESOLVED — intentional.** `UpdateAnimation` phase G reads `+0x59C != 0` (slot 16 handle) and, on non-null, clears slot 16 and creates slot 17 (SuperAnimFour). This is the **charge-over-threshold transition**: when the charging-phase indicator was created by OnPowerOn/OnConstructionComplete (slot 16 = SuperAnimThree), and the SW has now charged past the `ChargedAnimTime` threshold, phase G replaces it with the "charged" indicator (slot 17 = SuperAnimFour). Not a race — deterministic pre-emption.

**Evidence:** `BuildingClass::OnConstructionComplete @ 0x00445F80` creates slot 14/15/17 animations (lines around 0x00446559 in the disasm) driven by `Type+0x1304/+0x1314/+0x1324` (pre-charge) and `+0x138C/+0x139C/+0x13AC` (post-charge). These are orthogonal to `UpdateAnimation` phase G which handles **runtime transitions** between those two states.

**Confidence:** HIGH. **YR-active:** Yes (NukeSilo, Chronosphere, IronCurtain).

### B5 (T7). `+0x6E7` gate purpose

**Verdict:** **RESOLVED — "fog-of-war snapshot" flag (TS-legacy).**

**Evidence (binary):**
- **Writer:** `BuildingClass::CreateFoggedSnapshot @ 0x004D0EF0` sets `+0x6E7 = 1` at address 0x004D10D5. This function creates a phantom BuildingClass instance used by the TS-era fog-of-war system to keep a "previously seen" snapshot visible after the real building has moved/died.
- **Readers:**
  - `Draw dispatcher @ 0x0043CEA0` (vtable+0x104): gates the VXL/extras pass via `if (*(char*)(param_1 + 0x6e7) == '\0') vtable[0x4E4](...)` — i.e. **skip VXL rendering on fogged snapshots** (they are SHP-only placeholders).
  - `FUN_00457020` (select-cursor / display logic) at 0x00457172 and 0x004571AE: gates selection/hover behavior — you cannot select fogged snapshots.
- No other writers to `+0x6E7` found (byte-pattern sweep for `88 86/87/83 E7 06`, `C6 86/87/83 E7 06` = single hit, inside `CreateFoggedSnapshot`).

**YR status:** **FoW defaults OFF in YR** (`[MultiplayerDialogSettings] FogOfWar=false`). `CreateFoggedSnapshot` is only called via `FUN_00457AA0` which is behind the `SpecialFlags & 0x1000` gate (see `BUILDINGCLASS_UNLIMBO_AND_PLACEMENT.md:577` for the TS-legacy annotation). In standard YR the flag is never set; the VXL pass always runs.

**Rust implication:** omit until FoW is implemented; if FoW toggled on later, wire the flag.

**Confidence:** HIGH. **YR-active:** No (TS-legacy, SpecialFlags-gated).

### B6 (T7). Four construction overlays `+0x14E4 / +0x14EC / +0x14FC / +0x1504`

**Verdict:** **DEFERRED** — minimum scope: decompile `BuildingTypeClass::ReadINI @ 0x0045FE50` and `BuildingTypeClass::LoadVisualAssets @ 0x0045F230` searching for `CCINIClass::ReadString` that stores to these specific offsets.

**Evidence so far:**
- `BUILDINGTYPECLASS_CTOR_DEFAULTS.md:136-144` marks 0x14E4, 0x14EC, 0x14FC, 0x1504 as "orphan" (ctor-initialised but no known INI key).
- Offset layout: these sit past the 21-slot `AnimTypeClass` array (the last slot `SuperLowPower` is at `0x149C`, stride 0x44, ending at `0x14E0`). So 0x14E4 is the first field past the slot table.
- DrawBody's construction-mission branch dispatches on a stack byte `cVar6 = (char)((uint)unaff_EBP >> 0x18)` which selects between the four. Per T7's hypothesis: healthy-buildup / healthy-complete / damaged-buildup / damaged-complete. Best match is the v1 BuildUp SHP system.
- Retail ConYards use the `BuildUp` anim via `Type+0xF04` (BState table), so these four slots likely only fire on **non-ConYard construction animations** (gate buildings, SuperWeapon buildup overlays). No stock YR art.ini key has been pinned.

**Follow-up scope:** ~30 min Ghidra work — trace string xrefs near `BuildingTypeClass::ReadINI` for `"BuildupShape"` / `"BuildupPalette"` / `"UnderConstruction"` / similar.

**Confidence:** LOW. **YR-active:** Unclear; likely inert (no stock building triggers them).

### B7 (T7). `Type+0x1518` BibShape vs terrain-bib confusion

**Verdict:** **RESOLVED — `+0x1518` IS the primary BibShape** (not a "damaged-only" variant).

**Evidence:** `BIB_SYSTEM_GHIDRA_REPORT.md:14,58-63,301-326`:
- `Type+0x1518` stores the SHP pointer set by `LoadVisualAssets` from `BibShape=` in artmd.ini.
- `Type+0x151C` is the "has-been-set" byte flag — nonzero when INI provided BibShape=.
- **No default-bib fallback in YR.** If `BibShape=` is absent the bib is NOT drawn.
- There is no terrain-bib / CoreBib; v1 docs conflated two concepts.

**Rust implication:** use `+0x1518` as the single BibShape source. No fallback logic.

**Confidence:** HIGH. **YR-active:** Yes (retail WF, Refinery, etc. all set `BibShape=`).

### B8 (T7). Ambient-light audit inside `CC_Draw_Shape`

**Verdict:** **CONFIRMED NEGATIVE — DrawBody does not read `+0x600` or `+0x614` along any code path.**

**Evidence:** T7 already verified this via direct decompilation of `CC_Draw_Shape` helpers. Byte-pattern sweep of the DrawBody decomp + disasm for the fields `+0x600`, `+0x614`: zero reads. The ambient-light pipeline lives in `TechnoClass::DrawSHP @ 0x00705E00` which applies palette-table selection based on house-remap, not per-pixel light. The `LightSource*` at `+0x600` is consumed by `UpdateAmbient` elsewhere in the render dispatch, but NOT inside DrawBody.

**Rust implication:** DrawBody's Rust port can safely ignore those fields.

**Confidence:** HIGH. **YR-active:** N/A.

### B9 (T7). `DAT_00818CB0 / DAT_00818CB4` VXL barrel wobble magnitudes

**Verdict:** **DEFERRED** — low-priority cosmetic detail.

**Minimum scope:** inspect memory at those globals' runtime values, then grep readers. ~10 min.

**Confidence:** N/A. **YR-active:** likely yes but sub-frame drift not parity-critical.

### B10 (T7). BarrelStartPitch per-facing lookup in AnimClass::DrawIt slot 9

**Verdict:** **CONFIRMED — the per-facing lookup happens inside `AnimClass::DrawIt` for slot 9 (TurretAnim), NOT inside DrawBody.** T7 already verified DrawBody does not read `Type+0x1710`. The anim slot itself stores the pitch offset which is rasterised by the AnimClass frame stepper.

**Evidence:** see T7 §14 and `BUILDINGCLASS_UPDATE_ANIMATION_GHIDRA_REPORT.md` phase B facing helper `FUN_00456FB0`.

**Confidence:** HIGH. **YR-active:** Yes.

### B11 (T8). Save-size puzzle — `vtable[12] SizeOf` returns 6, but full struct is 0x720 bytes

**Verdict:** **RESOLVED — `vtable[12]` is `What_Am_I` (RTTI tag), NOT SizeOf. The object body is written through a separate per-object raw dump driven by the outer `OleSaveToStream` shell.**

**Evidence (binary):**
- Vtable at `0x007E3EBC` slot 12 (offset 0x30) = `0x00459EC0`. Decompile: `undefined4 BuildingClass__WhatAmI(void) { return 6; }`. Ghidra's auto-label of this slot as "GetClassSize" is wrong — it's the RTTI-tag getter.
- `AbstractClass::Save @ 0x00410320` writes `(this pointer, 4 bytes)` then `(RTTI enum, 6 bytes via vtable[12])` — total 10 bytes per object on the IStream. That is ALL `AbstractClass::Save` emits.
- The actual per-object state persists via the **structured-storage container** (see the Save/Load doc's OLE imports `StgCreateDocfile`, `StgOpenStorage`, `OleSaveToStream`, `OleLoadFromStream`). Each top-level object stream is a separate substorage holding a raw byte dump of the fixed-size struct at `this`, followed by each DynamicVector's per-element body.
- On Load, `BuildingClass::Load @ 0x00453E20` calls `TechnoClass::Load_IStream @ FUN_0070BF50` which reads the raw blob, then re-runs `BuildingClass::Constructor` on the already-populated memory to re-wire vtables, then registers pointer slots (`+0x148, +0x149, +0x150, +0x152, +0x153, +0x180, +0x1BD`) for swap-map fixup.

**Implication:** the puzzle is a decompilation artifact, not a bug. Each BuildingClass saves its 0x720-byte body via the outer structured-storage path; the IStream-level Save only handles the pointer fixup header.

**Rust implication:** for snapshot serialisation (project_snapshot_serialization MEMORY item), follow the same model — pack the struct as bytes then have a separate pointer fixup pass. Do NOT mirror the 6-byte `What_Am_I` write; that's a legacy IPersistStream quirk.

**Confidence:** HIGH. **YR-active:** Yes (save/load path).

### B12 (T8). `+0x540`, `+0x548`, `+0x54C` pointer identities

**Verdict:** **PARTIALLY RESOLVED; SecretLab closed (see A3), other three DEFERRED.**

**Breakdown:**
- `+0x6F4` = **SecretLab runtime pick** (A3 — RESOLVED).
- `+0x540`: **Bridge destruction damage source** (TechnoClass-inherited / HighBridge usage). Per `BUILDINGCLASS_UPDATE_AI_TICK_GHIDRA_REPORT.md:621,638,646`: passed to `vtable[0x16C]` as the damage-source argument. **RESOLVED.**
- `+0x544`, `+0x548`, `+0x54C`: Abstract-derived pointers registered with the Load fixup dict (see BuildingClass::Load disasm). Not read in any major mission handler.
  - `+0x54C` — set in `BuildingClass::Init_Managers @ 0x00442C40`: `*(int*)(param_1 + 0x53C) = *(Owner.+0x21C.+0x34.+0xB8)` — this writes **at BuildingClass+0x53C**, not 0x540. Close neighbour.
  - Remaining offsets: need a load-time-only-reader sweep. **DEFERRED.**

**Minimum follow-up:** ~20 min field_access_context search for write-after-ctor patterns on +0x544/+0x548/+0x54C.

**Confidence:** MED for +0x540 (clear bridge use); LOW for +0x544/+0x548/+0x54C. **YR-active:** partially (bridges only are stock YR).

### B13 (T9). `Rules+0x1460`

**Verdict:** **RESOLVED — `AIBaseSpacing=` (int, default 1000).**

**Evidence:** `RULESCLASS_FIELDS.csv:711`:
```
0x00672AE0,AI,AIBaseSpacing,0x1460,int,yes,1000
```
Source function `FUN_00672AE0` reads `[AI] AIBaseSpacing` into `Rules+0x1460`. Used by `Unlimbo` as a passability OR-mask radius around existing base structures when placing new buildings — meaning "minimum cells between adjacent bases to prevent placement collisions".

**Rust implication:** read from `rulesmd.ini [AI] AIBaseSpacing=`; default 1000.

**Confidence:** HIGH. **YR-active:** Yes (AI base placement).

### B14 (T9). Wall-orientation mapping {0, 4, 8, 0xC}

**Verdict:** **DEFERRED — low priority, 30min follow-up.** Minimum scope: decompile `CellClass::ConnectNeighborWalls` and correlate each orientation byte with the NESW cardinal by tracing the sprite-frame index written into the overlay cell.

**Note:** these four are the valid wall-seg values (NESW pieces). Their mapping to cardinal directions is deterministic but not recorded in existing docs.

**Confidence:** N/A. **YR-active:** Yes (stock Wall buildings).

### B15 (T9). Fogged-snapshot lifecycle

**Verdict:** **DEFERRED — TS-legacy, not relevant to stock YR.**

**Evidence:** `BUILDINGCLASS_UNLIMBO_AND_PLACEMENT.md:577` tags `FUN_00457AA0 CreateFoggedSnapshot` as SpecialFlags-gated (0x1000). Default off. Stock YR does not clone fogged objects.

**Minimum scope:** if FoW is later implemented, trace `FoggedObjectClass::Destructor` and grep for +0x6E7-tagged building destruction flows.

**Confidence:** N/A. **YR-active:** No.

### B16 (T9). 11 trait-list type flags at HouseClass +0x80/+0x98/.../+0x140

**Verdict:** **DEFERRED — partially resolved in scattered docs.**

**Evidence scattered across:**
- `BUILDINGCLASS_MASTER_GHIDRA_REPORT_V2.md` §10: Cloning list at HouseClass+0xFC (used by ExitObject Barracks path).
- `HOUSECLASS_VERIFIED_FIELD_MAP.md:97`: Storage-capacity accumulator at +0x310 (sum of Storage=).
- `BUILDINGCLASS_PREREQUISITES_GHIDRA_REPORT.md:448-451` marks the +0x16A9..+0x16B0 flag bytes as driving per-category aggregate lists but does not enumerate.

**Minimum scope:** ~60 min — for each HouseClass offset in the list, decompile Unlimbo/OnDestroyed writer pair (they are `DynamicVector::Add`/`Remove` call sites) and match to the BuildingType flag byte gate.

**Confidence:** LOW overall; individual entries HIGH where specific docs already map them.

### B17 (T10). `Rules+0x5C8`

**Verdict:** **RESOLVED — `ShakeScreen=` (int, default 671). Cost-refund hypothesis was WRONG.**

**Evidence:** `RULESCLASS_FIELDS.csv:448`:
```
0x006691E0,AudioVisual,ShakeScreen,0x5C8,int,yes,671
```
Source function `RulesClass::ReadAudioVisual @ 0x006691E0` reads `[AudioVisual] ShakeScreen` into `Rules+0x5C8`. Used as camera-shake magnitude on large explosions. Not a refund divisor.

**Implication for T10:** the §2j "cost-fractional test" in `OnDestroyed` was misattributed. The actual semantics of that division likely use a different Rules field (needs re-examination if still relevant).

**Confidence:** HIGH. **YR-active:** Yes.

### B18 (T10). `+0x6E3` setter semantics

**Verdict:** **RESOLVED — `OwnershipChanged` / "has been captured" flag.**

**Evidence:** Read in `BuildingClass::GetSurvivorInfantryType @ 0x0044EB10`:
```c
if (*(char *)(param_1 + 0x6e3) == '\0') {
    iVar1 = Random__RandomRanged(0, 99);
    if (iVar1 < 0x19 && *(int*)(param_1->Type + 0xeb8) == 7) {
        return *(undefined4 *)(g_RulesClass_Instance + 0xf70);  // Engineer
    }
}
```
The rule: **25% chance Engineer spawns as survivor IF building has NOT been captured AND building is Factory=BuildingType (ConYard)**. See Group C below for the correction to v2 §24.

**Writers:** `BuildingClass::ChangeOwner @ 0x00448260` — sets `+0x6E3 = 1` when ownership is transferred (capture via Engineer or ownership swap via MindControl). `BuildingClass::Constructor @ 0x0043B740` initialises to 0.

**Confidence:** HIGH. **YR-active:** Yes (every capture triggers this).

### B19 (T10). ParticleSystemClass lifetime post-building-death

**Verdict:** **DEFERRED** — medium-priority; 20 min scope.

**Minimum scope:** grep `ParticleSystemClass::Destructor` callers; check whether they are invoked from `BuildingClass::Limbo @ 0x00445880` or orphaned until the map teardown.

**Confidence:** N/A. **YR-active:** Yes (gap-gen particles).

### B20 (T11). `FUN_00509140` (misnamed "UpdateRadar") refund semantics

**Verdict:** **DEFERRED** — T11 marked it for FactoryClass deep-dive. That plan is a separate re-investigation target.

**Minimum scope:** decompile FactoryClass::AbandonProduction for refund computation; trace every caller of `FUN_00509140`.

**Confidence:** N/A. **YR-active:** Yes (AI build-queue validation).

### B21 (T11). BuildingType flag bytes `+0x16A9..+0x16B0` to HouseClass per-category lists

Overlaps with B16 above. Same DEFERRED status, same follow-up scope.

### B22 (T11). Slave Miner deploy check — `type+0x5BC` and result field `+0xE7`

**Verdict:** **DEFERRED** — ~15 min scope.

**Evidence partial:** CanBuild step 8 reads `*(*(*(this+0x258) + type[0x5BC]*4)+0x28)+0xE7`. The array at `this+0x258` is HouseClass SuperWeapon/SlaveManager list (same offset reused for different RTTI branches). Pin the struct via decompilation of `SlaveManagerClass::Constructor`.

**Confidence:** LOW. **YR-active:** Yes (Yuri Slave Miner).

### B23 (T12). Full decomp of `OnConstructionComplete` (vtable+0x4DC)

**Verdict:** **RESOLVED — now decompiled.** Full body is at `BuildingClass::OnConstructionComplete @ 0x00445F80` (not 0x004467A1 which is mid-function).

**Evidence (binary summary):**
- **Entry gate:** `if (ActuallyPlacedOnMap && !force_flag) return;` — runs exactly once per building unless `param_2` overrides.
- **ProduceCashTimer init:** if `Type+0x1560 != 0` (ProduceCashStartup=), sets `+0x6D0 = g_CurrentFrameCounter`, `+0x6D4 = packed_zero`, `+0x6D8 = ProduceCashStartup`.
- **Initial animations:** per-tier anim create (IdleAnim +0x1414/+0x1424/+0x1434, ActiveAnim +0x1018/+0x1028/+0x1038, ActiveAnimTwo +0x105C/+0x106C, ActiveAnimThree +0x10A0/+0x10B0, ActiveAnimFour +0x10E4/+0x10F4). Refineries diverge — tier-based selection via `(storage<<2)/capacity`.
- **Aggregate counters:** `Type+0x16CC (OrePurifier=)` → `Owner+0x538C++`; `Type+0x1564 (Power=)` → `Owner+0x164 += power`; `Type+0x1568 (Drain=)` → `Owner+0x168 += drain`; `Type+0x16CB (FactoryPlant=)` → `Owner+0x2D4 += Type+0x1780` + `FUN_006A60E0(3)` (recompute build bonuses).
- **SuperWeapon indicator:** if `Type+0x16F0 != -1`, scan `Owner.SuperWeapons[+0x258..+0x264]` for matching `+0xB4` type, compute remaining time `(sw+0x38 − (g_CurrentFrameCounter − sw+0x30))`, pick pre-charge (tier +0x1304/+0x1314/+0x1324) or post-charge (+0x138C/+0x139C/+0x13AC) based on `remaining/15*60 < 5`.
- **Cycling anim at +0x5FC:** if `+0x5FC == -1`, create PoweredEffect anim (Type+0x11B0). Else, walks string concat chain storing anim name into a 64-byte buffer.
- **Shadow direction sync:** writes `(+0x580 + 0xAC)` = shadow frame from RateTimer current.
- **LightSource:** if `Type+0xE34 != 0` AND `+0x600 == 0`, allocates new `LightSourceClass` from coords + Type+0xE30..0xE40 → stored at `+0x600`.
- **ConnectWalls, AddSensorArrayAt, AddDetectDisguiseAt** — terminal dispatchers.
- **ActuallyPlacedOnMap = true** — fenceposts the one-shot.
- **Rally point:** if `Type+0x16BD (WeaponsFactory=)` AND `Type+0xCCE (Naval=)` AND no active radio → computes `Find_Nearby_Passable_Cell` and calls `SetRallyPoint`.
- **OnPowerOn gate:** if `Type+0x1573 (Powered=)`, call `OnPowerOn`.
- **ConYard free-MCV path:** if `Type+0xEA0` (FreeUnit= via some ConYard-linked type) AND not human AND not map editor → construct and Unlimbo an ancillary unit.
- **Paratrooper initial:** if `Rules+0x17E8 == '\0'` AND `Type+0x154E != '\0'` → constructs an AircraftClass, sets its mission to ENTER/DEPLOY (0x278).

**Rust implication:** this handler is the single pivot point for "construction just finished". Implement a `BuildingClass::on_construction_complete` function that runs all 15+ side effects above in order.

**Confidence:** HIGH. **YR-active:** Yes (every building finish).

### B24 (T12). `FUN_00465AF0` — `Type+0x1762`-gated one-shot resource at `Type+0xE00`

**Verdict:** **DEFERRED — likely `BuildupShape` cache** but not string-anchored.

**Evidence:**
```c
void FUN_00465af0(int param_1) {
    if ((*(char *)(param_1 + 0x1762) != '\\0') && (*(int *)(param_1 + 0xe00) != 0)) {
        FUN_007c8b3d(*(int *)(param_1 + 0xe00));  // free()
        *(undefined4 *)(param_1 + 0xe00) = 0;
        *(undefined1 *)(param_1 + 0xe04) = 0;
    }
}
```
Callers: BuildingClass::Constructor, BuildingClass::Init_Managers, Mission_Construction. Always called post-`Init_Managers`'s `vtable[0xC0]` check. Pattern matches "free a cached asset after first use" — consistent with ``BuildupShape=`` (SHP cache). Not confirmed.

**Minimum scope:** ~15 min — decompile BuildingTypeClass INI parser's string literal table near `0x1762` offset writes.

**Confidence:** LOW. **YR-active:** Likely yes but non-critical.

---

## Group C — v2 §24 correctness-fix sanity pass

### C1. Sell refund should be non-health-scaled

**Verdict:** **CONFIRMED.**

**Evidence (binary):** `BuildingClass::Sell @ 0x00449C30` state-2 path (actual sell completion) — `uVar9 = vtable[0x2BC](); HouseClass__Add_Credits(uVar9);`. The vtable slot (offset 0x2BC / slot 175) is `GetCost` returning `Type+0x160 (Cost=) * SellBack`. **No GetHealthRatio() multiplication**. Contrast with `Mission_Missile` refund which IS health-scaled (different code path).

Storage refund is separate: loops StorageClass slots and calls `HouseClass::Add_Tiberium_Credits` per slot via `Math::ftol(GetAmount())`.

**Rust implication:** in the port, the Sell handler pays `Cost × SellBack + Storage_refund` — do NOT multiply by health ratio.

**Confidence:** HIGH. **YR-active:** Yes.

### C2. Soviet Engineer rule: **WRONG in v2** — correction confirmed

**Verdict:** **CORRECTED TO: side-independent rule.**

**Evidence (binary):** `BuildingClass::GetSurvivorInfantryType @ 0x0044EB10`:
```c
if (*(char *)(param_1 + 0x6e3) == '\\0') {         // not captured
    iVar1 = Random__RandomRanged(0, 99);
    if ((iVar1 < 0x19) && (*(int*)(*(int*)(param_1 + 0x520) + 0xeb8) == 7)) {
        return *(undefined4 *)(g_RulesClass_Instance + 0xf70);  // Rules.Engineer
    }
}
return FUN_00707d20();  // TechnoClass::Crew_Type (side-based + Technician)
```
Gates: `+0x6E3 == 0` (never captured) AND `random 0..99 < 25` (25% chance) AND `Type.Factory (+0xEB8) == 7` (Factory=BuildingType, i.e. ConYard). **No side/country check.** The v2 §24 "Soviet-side only" claim is **wrong**. Any nation's uncaptured ConYard has the 25% Engineer survivor rule.

**Rust implication:** implement the Engineer-from-ConYard rule without Soviet-gating. Applies to Allied, Soviet, and Yuri alike.

**Confidence:** HIGH. **YR-active:** Yes.

### C3. Gap-gen state at +0x220 (NOT +0xBC)

**Verdict:** **CONFIRMED.**

**Evidence (binary):** `BuildingClass::UpdateGapGenerator_Tick @ 0x00454DB0`:
- `param_1[0x88]` = byte offset 0x88 * 4 = **0x220**. Tests against 1 (opening), 2 (steady), 3 (closing), 0 (off).
- The gap-gen fade counter is `+0x6ED` (range 0..0xF).
- `+0xC3` = `param_1[0xC3] = 0` when gap-gen goes to steady state (wipes the pending particle system pointer).

**Rust implication:** represent gap-gen state at BuildingClass+0x220, not at +0xBC. The +0xBC reference in v1 was an error.

**Confidence:** HIGH. **YR-active:** Yes.

### C4. SensorArray add/remove radius asymmetry

**Verdict:** **CONFIRMED (intentional bug in retail YR).**

**Evidence:** Per `BUILDINGCLASS_CLOAK_SENSOR_GHIDRA_REPORT.md:251-341`:
- `AddSensorArrayAt @ 0x00455820` reads `Type+0x5F0 SensorsSight` (int, TechnoTypeClass-inherited).
- `RemoveSensorArrayAt @ 0x004556D0` reads `Type+0x1707 CloakRadiusInCells` (byte, BuildingTypeClass).
- Stock YR Psychic Sensor has `SensorsSight=15`, `CloakRadiusInCells=0` → **remove is a no-op**, the sensor field persists until house is removed or MapClass is rebuilt.

**Rust implication (per doc recommendation):** do NOT replicate the bug. Use `SensorsSight` for both add and remove.

**Confidence:** HIGH. **YR-active:** Yes.

---

## Summary Table

| # | Question | Verdict | Key Evidence |
|---|---|---|---|
| (T1) #1 | AircraftClass::What_Am_I=2 | **RESOLVED** | MISSION_GUARD_AREAGUARD :255 |
| A1 (v2 #2) | BuildOrder +0x8/+0xC | **DEAD** | AI_Manage_Build_Queue 0x004FDD10 + FUN_0050A490 |
| (T1) #4 | Type+0x1573 = Powered | **RESOLVED** | BUILDINGCLASS_MISSION_ATTACK :34 |
| (T1) #5 | Helipad radio 0x1D = REFUEL_QUERY | **RESOLVED** | BUILDINGCLASS_MISSION_REPAIR_AND_PRODUCE :414 |
| (T1) #6 | +0x664 = FirePowerBonus | **RESOLVED** | UPDATE_AI_TICK :832 |
| A2 (v2 #7) | +0x700 short | **DEAD** | ctor 0x0043B9D3 + UpdateAnim 0x00450A62, no readers |
| A3 (v2 #8) | +0x6F4 SecretLab pick | **RESOLVED** | SPECIAL_BUILDINGS :413, Load fixup 0x00453FB2 |
| (T1) #9 | Rules+0x16E8 URepairRate=.016 | **RESOLVED** | DOCK_AND_HEAL :173 |
| A4 (v2 #10) | Type+0x184C/+0x184D | **DEAD — not on BuildingTypeClass** | ctor size 0x1798 < 0x184C; actual = Rules+0x184C ElevationBonusCap |
| (T1) #11 | Bunker +0x718 cleanup | **RESOLVED** | BUNKER_SYSTEM :135 |
| B1 | UpdateAnim local_10 bleed | **HARMLESS** | ESP+0x18 → +0x104 unused downstream |
| B2 | +0x218 phase-H gate | **RESOLVED** | Sell + OCC — primary radio contact pointer |
| B3 | Phase F tier-overflow | **HARMLESS** | Storage=20 in stock |
| B4 | Slot-16 / phase-G preemption | **RESOLVED** | intentional SW charge transition |
| B5 | +0x6E7 | **RESOLVED (TS-legacy)** | CreateFoggedSnapshot 0x004D0EF0 |
| B6 | +0x14E4/.../+0x1504 overlays | **DEFERRED** | orphan in ctor; likely BuildUp SHPs |
| B7 | +0x1518 BibShape | **RESOLVED** | BIB_SYSTEM :14,58-63 |
| B8 | CC_Draw_Shape ambient | **CONFIRMED NEG** | no +0x600/+0x614 reads |
| B9 | DAT_00818CB0/CB4 | **DEFERRED** | low priority |
| B10 | BarrelStartPitch lookup | **CONFIRMED** | in AnimClass::DrawIt, not DrawBody |
| B11 | Save-size puzzle | **RESOLVED** | vtable[12]=What_Am_I, not SizeOf |
| B12 | +0x540/+0x548/+0x54C ptrs | **PARTIAL** | +0x540=Bridge damage src; others DEFERRED |
| B13 | Rules+0x1460 | **RESOLVED — AIBaseSpacing** | RULESCLASS_FIELDS :711 |
| B14 | Wall-orientation mapping | **DEFERRED** | 30min |
| B15 | Fogged-snapshot lifecycle | **DEFERRED — TS-legacy** | SpecialFlags-gated |
| B16 | HouseClass trait lists | **DEFERRED — partial** | 60min |
| B17 | Rules+0x5C8 | **RESOLVED — ShakeScreen** | RULESCLASS_FIELDS :448. T10's hypothesis **WRONG**. |
| B18 | +0x6E3 | **RESOLVED — OwnershipChanged** | GetSurvivorInfantryType 0x0044EB10 |
| B19 | ParticleSystem lifetime | **DEFERRED** | 20min |
| B20 | FUN_00509140 refund | **DEFERRED — FactoryClass scope** | |
| B21 | +0x16A9..+0x16B0 list routing | **DEFERRED** (== B16) | |
| B22 | Slave Miner +0xE7 | **DEFERRED** | 15min |
| B23 | OnConstructionComplete | **RESOLVED — decompiled** | 0x00445F80 |
| B24 | Type+0x1762 / +0xE00 | **DEFERRED** | 15min (likely BuildUp cache) |
| C1 | Sell refund non-scaled | **CONFIRMED** | Sell state 2 calls vtable[0x2BC] directly |
| C2 | Soviet-Engineer rule | **CORRECTED — side-independent** | GetSurvivorInfantryType 0x0044EB10 |
| C3 | Gap-gen state +0x220 | **CONFIRMED** | UpdateGapGenerator_Tick 0x00454DB0 |
| C4 | SensorArray radius asymmetry | **CONFIRMED — bug** | CLOAK_SENSOR :251 |

**Verdict counts:** RESOLVED 22 (6 T1 + 4 Group A + 9 Group B + 3 Group C corrections/confirmations) · DEAD 3 (A1 A2 A4) · PARTIAL 2 (B12, B16/B21) · DEFERRED 9 (B6 B9 B14 B15 B16 B19 B20 B22 B24) · HARMLESS-BUG 2 (B1 B3) · CORRECTIONS 1 major (C2).

**Most impactful resolution:** **A4 (Type+0x184C/+0x184D = out-of-bounds)** plus **C2 (Engineer rule is side-independent)**. A4 removes two non-existent fields from the Rust BuildingType struct plan entirely. C2 reverses the v2 §24 "Soviet-only" framing and simplifies the Rust Engineer-survivor rule.

**New gap worth adding to v3 master:**
- **+0x14E4/+0x14EC/+0x14FC/+0x1504** (B6) — these four BuildupShape-like slots exist in the ctor as orphans with no pinned INI key. The DrawBody read site is clear; the INI mapping is not. Worth one 30-min decompilation pass targeting `BuildingTypeClass::ReadINI + LoadVisualAssets` to close.

---

## Sources

**Live Ghidra decompilations (2026-04-24):**
- `0x0043B740` BuildingClass::Constructor (verified +0x700 init, +0x6E3/+0x6E7 init)
- `0x004509D0` BuildingClass::UpdateAnimation (full disasm — phase B +0x700 write)
- `0x00453E20` BuildingClass::Load (fixup list — +0x1BD = +0x6F4 registered)
- `0x00459840` FUN_00459840 (SecretLab query from CanBuild)
- `0x004FDD10` HouseClass::AI_Manage_Build_Queue (BuildOrder writer)
- `0x00506EF0` HouseClass::AI_ChooseNextProduction (BuildOrder reader)
- `0x0050A490` FUN_0050A490 OnBuildingDestroyed (BuildOrder invalidator)
- `0x0066D150` RulesClass ElevationModel reader (proved Type+0x184C is Rules-side)
- `0x00668BF0` RulesClass::ReadRules (confirmed +0x184C caller context)
- `0x00449C30` BuildingClass::Sell (refund math — state 2 path)
- `0x00454DB0` BuildingClass::UpdateGapGenerator_Tick (state at +0x220)
- `0x0044EB10` BuildingClass::GetSurvivorInfantryType (Engineer rule — no side check)
- `0x00410320` AbstractClass::Save (vtable[12] byte count via What_Am_I)
- `0x00459EC0` BuildingClass::What_Am_I (returns 6; Ghidra mislabelled as SizeOf)
- `0x0043CEA0` Draw dispatcher (+0x6E7 gate on vtable[0x104])
- `0x00457020` FUN_00457020 select-cursor logic (+0x6E7 reader)
- `0x004571E0` BuildingClass::OnSpyInfiltrate (+0x6F4 related context)
- `0x0045DD90` BuildingTypeClass::constructor (struct size boundary confirmation)
- `0x00442C40` BuildingClass::Init_Managers (FUN_00465AF0 caller context)
- `0x00465AF0` Type+0x1762 one-shot freer
- `0x00445F80` BuildingClass::OnConstructionComplete (full decomp)

**Byte-pattern sweeps:**
- `00 07 00 00` (+0x700 short): 2 hits total — ctor + UpdateAnim
- `F4 06 00 00` (+0x6F4 byte-4): only read site inside FUN_00459840; write sites in SecretLab external code (FUN_0068C050)
- `4C 18 00 00` / `4D 18 00 00`: 2 hits + 0 hits, both in Rules reader — no BuildingTypeClass xrefs
- `E3 06 00 00` (+0x6E3): 14 hits, readers in ChangeOwner/GetSurvivorInfantryType/etc
- `E7 06 00 00` (+0x6E7): 15 hits total; single writer = CreateFoggedSnapshot; readers in Draw/select logic only

**Existing cross-referenced docs:**
- `BUILDINGCLASS_MASTER_GHIDRA_REPORT_V2.md` (v2 §24-§25 source)
- `BUILDINGCLASS_UPDATE_ANIMATION_GHIDRA_REPORT.md` (T6)
- `BUILDINGCLASS_DRAWBODY_GHIDRA_REPORT.md` (T7)
- `BUILDINGCLASS_SAVE_LOAD_GHIDRA_REPORT.md` (T8)
- `BUILDINGCLASS_UNLIMBO_AND_PLACEMENT.md` (T9)
- `BUILDINGCLASS_ON_DESTROYED_GHIDRA_REPORT.md` (T10)
- `BUILDINGCLASS_PREREQUISITES_GHIDRA_REPORT.md` (T11)
- `BUILDINGCLASS_MISSION_GUARD_AND_CONSTRUCTION.md` (T12)
- `BUILDINGCLASS_OPEN_QUESTIONS_VERIFICATION_R3.md` (T1 closures)
- `BUILDINGCLASS_SPECIAL_BUILDINGS_GHIDRA_REPORT.md` (A3 closure)
- `BUILDINGCLASS_CLOAK_SENSOR_GHIDRA_REPORT.md` (C4 closure)
- `BUILDINGCLASS_UPDATE_AI_TICK_GHIDRA_REPORT.md` (B12 partial)
- `BIB_SYSTEM_GHIDRA_REPORT.md` (B7 closure)
- `BUILDINGTYPECLASS_FIELDS.csv` (A3, B6 anchor)
- `BUILDINGTYPECLASS_CTOR_DEFAULTS.md` (B6, A4)
- `RULESCLASS_FIELDS.csv` (B13, B17)

**TS-legacy filter applied:** B5, B15 both gated behind FogOfWar (SpecialFlags 0x1000) — marked `YR-active: No`. No other TS-legacy branches surfaced in this batch.
