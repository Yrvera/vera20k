# NumberOfDocks vs DockOffset Reconciliation
**Ghidra RE Report — BuildingTypeClass +0x1780/+0x1788 vs +0x1618/+0x161C**
Date: 2026-05-19
Scope: Resolve which dock-offset system the chrono miner harvest-return state 2 uses; reconcile two competing claims about BuildingTypeClass fields at +0x1618/+0x161C vs +0x1780/+0x1788.

---

## Verdict (read this first)

**There is no conflict.** The two sets of fields serve entirely different code paths:

| Field | INI Key | INI File | Purpose |
|-------|---------|----------|---------|
| +0x1618 (short) | `QueueingCell` X | art.ini | Harvester queuing/warp-landing cell offset X |
| +0x161C (short) | `QueueingCell` Y | art.ini | Harvester queuing/warp-landing cell offset Y |
| +0x1780 (int) | `NumberOfDocks` | rules.ini | Dock pad count for helipad/naval/repair buildings |
| +0x1788 (int*) | `DockingOffset%d` array | art.ini | Per-pad 3D coord offsets; used by `GetDockCoord` for Helipad/UnitRepair only |

The chrono miner harvest-return path (Mission_Harvest state 2) uses **only +0x1618/+0x161C**.
The +0x1788 DockingOffset array is **never touched** by any harvester or refinery code path.

---

## 1. Verified ReadINI Sites (BuildingTypeClass_ReadINI_Water @ 0x45FE50)

All offsets verified by grepping the Ghidra decompile result for the function at 0x45FE50.
`param_1` type is `int` throughout this function, so all offsets are direct byte offsets.

### (a) QueueingCell → +0x1618/+0x161C

```c
// ReadINI_Water, char position ~34947
uStack_2c0 = 0;
uStack_2bc = 0;
puVar9 = (undefined4 *)
         CCINIClass__ReadMinMax(&uStack_358, iVar15, s_QueueingCell_0081a614, &uStack_2c0);
uVar8 = puVar9[1];
*(undefined4 *)(param_1 + 0x1618) = *puVar9;   // QueueingCell X (short packed in int)
*(undefined4 *)(param_1 + 0x161C) = uVar8;      // QueueingCell Y (short packed in int)
```

**`iVar15`** at this call site = `param_1 + 0x1f8` (the art.ini section handle, set ~2000 chars earlier).
Confirmed: QueueingCell is read from **art.ini**.

Evidence: ReadINI site at ~char 34947 in decompile output; string xref at 0x0081a614 → from address 0x00461506 in BuildingTypeClass_ReadINI_Water.

### (b) NumberOfDocks → +0x1780

```c
// ReadINI_Water, char position ~168883
iVar21 = *(int *)(param_1 + 0x1780);
iVar15 = CCINIClass__ReadInt(param_1 + 0x24,   // <-- rules.ini section handle
                              s_NumberOfDocks_008194c4, iVar21);
*(int *)(param_1 + 0x1780) = iVar15;
```

`param_1 + 0x24` is the rules.ini section handle.
Confirmed: NumberOfDocks is read from **rules.ini**.

Evidence: ReadINI site at ~char 168883 in decompile output; string xref at 0x008194c4 → from address 0x00464938.

### (c) DockingOffset%d array → +0x1788

```c
// ReadINI_Water, char position ~169591
// After NumberOfDocks is set, array is resized if needed (capacity at +0x178c),
// then a loop reads each entry:
iVar21 = *(int *)(param_1 + 0x1780);  // NumberOfDocks count
iVar15 = 0;
if (0 < iVar21) {
  iVar11 = 0;
  do {
    FUN_007c8ef4(auStack_1ac, s_DockingOffset_d_008194b4, iVar15);  // sprintf "DockingOffset%d"
    puVar9 = (undefined4 *)(*(int *)(param_1 + 0x1788) + iVar11);  // ptr to slot in array
    CCINIClass__Read3Int(&uStack_2c0, param_1 + 0x1f8, auStack_1ac, &uStack_358);  // art.ini
    // stores {X,Y,Z} int triple into array slot
    iVar15 = iVar15 + 1;
    iVar11 = iVar11 + 0xc;  // stride = 12 bytes per entry
    iVar21 = *(int *)(param_1 + 0x1780);
  } while (iVar15 < iVar21);
}
```

`param_1 + 0x1f8` is the art.ini section handle.
`+0x1788` is a `DynamicVector`-managed data pointer; `+0x178c` is the capacity; `+0x1784` is the vtable.
Each entry is `{int X, int Y, int Z}` = 12 bytes in leptons.

Confirmed: DockingOffset%d is read from **art.ini**.

Evidence: ReadINI site at ~char 169591 in decompile output; string xref at 0x008194b4 → from address 0x004649b7.

---

## 2. GetDockCoord (0x447B20) — Which Offset Does Each Building Type Use?

From the verified BUILDING_DOCKING_SYSTEM_GHIDRA_REPORT.md §2 + §11 (audit 2026-05-11):

| Building Flag | GetDockCoord Path | Uses +0x1618/+0x161C? | Uses +0x1788 array? |
|---------------|------------------|-----------------------|---------------------|
| Refinery (+0x16BB) | Building center + 0x80 (128 lep) X | NO | NO |
| Weeder (+0x16BC) | Fixed cell offset (2E, 1S) | NO | NO |
| Bunker (+0x16AB) | Angle-based ±128 lep | NO | NO |
| Helipad (+0x16CB) | DockingOffset[slot_index] via +0x1788 | NO | **YES** |
| UnitRepair (+0x16A9) | DockingOffset[slot_index] via +0x1788 | NO | **YES** |
| Default | Building center | NO | NO |

The +0x1788 array is **only used** by Helipad and UnitRepair buildings (airfields, repair depots).
Refineries in GetDockCoord use building-center + 0x80 only.

---

## 3. Mission_Harvest State 2 — Exact Flow (UnitClass__Mission_Harvest @ 0x73E5E0)

Decompiled and verified. `cVar1 = *(char *)(TypeClass + 0xCD4)` = Teleporter flag.

```
STATE 2 (full harvester, seeking refinery):

  piVar3 = Find_Docking_Bay(arg3=0)   // first: normal search, respect reservation

  if (cVar1 == 0):  // NOT teleporter (regular HARV)
    if piVar3 != NULL AND dist <= Rules+0xD78 * 0x100:   // HarvesterTooFarDistance=5
      Transmit(2, refinery) -> reserve slot -> transition to state 3 (drive in)

  else:             // IS teleporter (CMIN, cVar1 != 0)
    if piVar3 != NULL AND dist <= Rules+0xD7C * 0x100:   // ChronoHarvTooFarDistance=50
      Transmit(2, refinery) -> reserve slot -> transition to state 3 (drive in)

  // FALLBACK (normal path didn't fire):
  g_MapEditorMode++
  piVar3 = Find_Docking_Bay(arg3=1)   // relaxed: ignore reservation
  g_MapEditorMode--
  if piVar3 != NULL AND (dist > 0x300 OR cVar1 != 0):
    sVar10 = building_cell.X
    sVar2  = building_cell.Y
    // *** HERE: QueueingCell (+0x1618/+0x161C) used as warp-landing offset ***
    target = (sVar10 + *(short*)(TypeClass + 0x1618),
              sVar2  + *(short*)(TypeClass + 0x161C))
    Find_Nearby_Passable_Cell(target) -> Set_Destination -> warp (if teleporter) or drive
```

Evidence: Decompile of UnitClass__Mission_Harvest @ 0x73E5E0 (full function output).
Confidence: HIGH — read directly from decompilation.

### (d) Does the chrono miner use +0x1618/+0x161C, or the +0x1788 array?

**+0x1618/+0x161C only.** The +0x1788 DockingOffset array is never read in Mission_Harvest, state 2, or any harvester code path. It is exclusively used by GetDockCoord for Helipad/UnitRepair buildings.

### Is +0x1618/+0x161C used in the NORMAL path or only the fallback?

**Fallback only.** The normal path (within-distance → `Transmit(2, refinery) → state 3`) does NOT read QueueingCell. QueueingCell is only used when:
- arg3=1 fallback fires (no dock with free reservation slot found), AND
- (dist > 3 cells [0x300 leptons] OR unit is Teleporter)

For the chrono miner (`Teleporter=yes`, `cVar1 != 0`), the second condition is always true in the fallback, so QueueingCell is always the warp-landing offset when the fallback triggers.

---

## 4. INI Values for Stock YR Refineries

### NumberOfDocks (rules.ini)

| Building | NumberOfDocks | Note |
|---------|--------------|------|
| GAREFN | 1 | Allied Ore Refinery |
| NAREFN | 1 | Soviet Ore Refinery |
| YAREFN | not set | Yuri Slave Miner (Deploys=YAREFN); no harvester docking |
| GAAIRC | 4 | Airfield — uses +0x1788 array (4 pad coords) |
| AMRADR | 4 | Radar dome / Airfield |
| GADEPT | 0 (not set) | Allied Service Depot |
| NADEPT | 1 | Soviet Service Depot |

All stock harvest-path refineries (GAREFN, NAREFN) have `NumberOfDocks=1`.
No stock YR refinery has NumberOfDocks > 1.
The multi-dock array (+0x1788) is never exercised for any refinery in stock YR.

### QueueingCell (art.ini)

| Building | QueueingCell | Meaning |
|---------|-------------|---------|
| GAREFN | 4,1 | 4 cells east, 1 south of building origin cell |
| NAREFN | 4,1 | same |
| YAREFN | not set | not applicable (slave miner, no QueueingCell) |

NAREFN has a commented-out `DockingOffset0=256,0,0` in art.ini — TS-era lefover, not active in YR.

---

## 5. Confirmed RulesClass Distance Thresholds

From RulesClass__ReadGeneral @ 0x670003 decompilation (confirmed in function header comment):

| RulesClass Offset | INI Key | Default | Description |
|------------------|---------|---------|-------------|
| +0xD78 | `HarvesterTooFarDistance` | 5 | Max cells before regular harvester uses fallback path |
| +0xD7C | `ChronoHarvTooFarDistance` | 50 | Max cells before chrono miner uses fallback path |

Evidence: `CCINIClass__ReadInt(PTR_s_General_007f0c9c, str_HarvesterTooFarDistance, *(param_1+0xD78))` and same for ChronoHarv at 0xD7C — confirmed in ReadGeneral decompile at char ~54474.

---

## 6. Questions (a)–(g) Answered

**(a) INI key for +0x1780?**
`NumberOfDocks` — read from rules.ini via CCINIClass__ReadInt. Evidence: ReadINI call at char ~168883, string at 0x008194c4.

**(b) INI key for +0x1788? Array layout?**
`DockingOffset%d` (0-indexed, built by sprintf) — read from art.ini via CCINIClass__Read3Int.
Array layout: each entry is `{int X, int Y, int Z}` in leptons, stride 12 bytes (0xC).
Pointer at +0x1788 is the DynamicVector data pointer; capacity at +0x178c; vtable at +0x1784.
Evidence: ReadINI loop at char ~169591.

**(c) INI key for +0x1618 and +0x161C?**
Both are `QueueingCell` — a MinMax pair read via CCINIClass__ReadMinMax from art.ini.
+0x1618 = X cell delta (short in int storage), +0x161C = Y cell delta.
Evidence: ReadINI call at char ~34947, with `iVar15 = param_1 + 0x1f8` (art.ini handle) confirmed ~2000 chars earlier.

**(d) Does chrono miner state 2 use +0x1618/+0x161C or index into +0x1788 array?**
`+0x1618/+0x161C only`. These are used in the fallback path of state 2. The +0x1788 array is never read in Mission_Harvest. See §3 above.

**(e) NumberOfDocks for stock YR refineries?**
Always 1 (GAREFN=1, NAREFN=1). The multi-dock DockingOffset array is never used for any refinery in stock YR. GAAIRC/AMRADR use it with NumberOfDocks=4.

**(f) Are any +0x1788 readers reachable from chrono-miner or harvester paths?**
No. The only reader of +0x1788 is `GetDockCoord` (0x447B20) in the Helipad/UnitRepair branch. `GetDockCoord` is never called from Mission_Harvest or Mission_Deploy_Building. The doc's claim (BUILDING_DOCKING_SYSTEM §2 audit) that GetDockCoord is used by refineries is wrong — the Refinery branch of GetDockCoord uses building-center+0x80 only.

**(g) Other buildings with NumberOfDocks > 1?**
GAAIRC=4, AMRADR=4. These use the +0x1788 array for aircraft pad coordinates. No refinery-type building (Refinery=yes, DockUnload=yes) has NumberOfDocks > 1 in stock YR.

---

## 7. The Two Claims Reconciled

**Claim A** (BUILDING_DOCKING_SYSTEM_GHIDRA_REPORT §1): NumberOfDocks at +0x1780, DockingOffset POINTER at +0x1788.
→ **CORRECT**. These fields exist exactly as documented. Used by Helipad/UnitRepair buildings (airfields, repair depots). Never used for refineries in stock YR.

**Claim B** (FIND_DOCKING_BAY_FALLBACK_ARG3 §8, CHRONO_MINER_OVERVIEW §4): DockOffset at +0x1618/+0x161C used by chrono miner warp-return.
→ **CORRECT**, but the field name is `QueueingCell` (not "DockOffset"). The FIND_DOCKING_BAY report's description of them as "dock XY offsets" is accurate in operational context (they define the landing target cell adjacent to the refinery) but the INI key is `QueueingCell`. The CHRONO_MINER_OVERVIEW uses the informal label "DockOffset (+0x1618/+0x161C)" which is misleading — the field's canonical name is QueueingCell.

**Root of the apparent conflict:** The CHRONO_MINER_OVERVIEW used the informal label "DockOffset" for `+0x1618/+0x161C`, which sounds like it should match the `DockingOffset` array at +0x1788. It doesn't. They are unrelated fields serving different code paths.

---

## 8. Rust Implementation Note

For the Rust implementation of Mission_Harvest state 2:

- Read `building_type.queuing_cell` (two shorts at +0x1618/+0x161C) as the warp-landing offset.
- `NumberOfDocks` (+0x1780) and `DockingOffset` array (+0x1788) are NOT used in the harvester path.
- The refinery's `GetDockCoord` returns building-center + 128-lep X (not QueueingCell).
- QueueingCell is ONLY the warp-target / queuing cell in the Mission_Harvest fallback path.
- All stock YR refineries have QueueingCell=(4,1) — 4 cells east, 1 south.

---

## 9. Confidence Summary

| Claim | Confidence | Evidence |
|-------|-----------|---------|
| +0x1618/+0x161C = QueueingCell, from art.ini | HIGH | Direct ReadINI decompile site at 0x45FE50, string xref 0x461506 |
| +0x1780 = NumberOfDocks, from rules.ini | HIGH | Direct ReadINI decompile site, string xref 0x464938 |
| +0x1788 = DockingOffset array ptr, from art.ini | HIGH | Direct ReadINI loop, string xref 0x4649b7 |
| Mission_Harvest state 2 uses only +0x1618/+0x161C | HIGH | Full decompile of 0x73E5E0 |
| +0x1788 not used in any harvester path | HIGH | No read of +0x1788 in Mission_Harvest or Mission_Deploy_Building |
| GAREFN/NAREFN NumberOfDocks=1 in YR | HIGH | rulesmd.ini lines verified |
| QueueingCell=(4,1) for GAREFN/NAREFN | HIGH | art.ini and artmd.ini lines verified with comment |
| RulesClass+0xD78=HarvesterTooFarDistance, +0xD7C=ChronoHarvTooFarDistance | HIGH | ReadGeneral decompile comment + ReadInt call sites at ~char 54474 |

Status: **COMPLETE**
