# BuildingClass::Update — Bridge-Repair-Hut Branch — Decode Doc

Address: `0x0043FB20`  
Scope: BridgeRepairHut C4-timer-expiry branch only (narrow; full Update body is ~2650 bytes).

## Summary

When a BridgeRepairHut building (`BuildingTypeClass+0x16B6 == true`) has a live C4 charge
(`this+0x6DF != 0`) and its C4 countdown timer expires (`g_CurrentFrameCounter >=
this+0x528 + this+0x530`), `BuildingClass::Update` enters the bridge-destruction dispatch
block. It scans a 5×5 cell neighbourhood around the hut's cell to determine whether any
adjacent cell holds a low-bridge tile or a `CellClass+0x44` value in range `[0x4A, 0x65]`
(high-bridge overlay). Based on the scan result it calls either
`MapClass::DestroyBridge_Low_OnHutDeath` (low bridge found) or
`MapClass::DestroyBridge_High_OnHutDeath` (high bridge or no low bridge found), then clears
`this+0x6DF` and `this+0x540`.

This is the observable entry-point for the "C4 on bridge repair hut" destroy-bridge mechanic.
The player plants C4 (via InfantryClass::PerCellProcess), the hut's `+0x6DF` flag is set and
`+0x528`/`+0x530` timer is armed; Update fires the bridge destruction once the timer elapses.

## Active in YR

**Yes.** Verified call chain:
- `BuildingClass::Update` is bound at vtable slot `0x007e3f18` (verified via
  `read_memory 0x007e3f18` → bytes `20 FB 43 00` = `0x0043FB20`). This vtable belongs to
  BuildingClass and is dispatched by the game-loop AI/object-tick system.
- `MapClass::DestroyBridge_Low_OnHutDeath` (0x00574C20) and
  `MapClass::DestroyBridge_High_OnHutDeath` (0x00574000) are both called from live in-scope
  callers: `BombClass::Detonate` and `BuildingClass::Update` (verified via
  `get_xrefs_to 0x00574C20` and `get_xrefs_to 0x00574000`).
- The BridgeRepairHut type flag (`BuildingTypeClass+0x16B6`) gates the entire block; it is
  a real in-game type used on bridge-hut buildings in YR skirmish maps.

No TS-only gating flags. Not behind `SpecialFlags`, `FogOfWar`, or any known dead-code guard.

## Decompilation Excerpt — Bridge-Repair-Hut Branch

Extracted from `decompile_function 0x0043FB20`. The full Update body is ~2650 bytes; only
the BridgeRepairHut branch is reproduced here.

```c
// ---- C4 timer check + BridgeRepairHut branch ----
// (entered only when this->field_0x6df != 0 and health > 0)

if (this->field_0x6df != '\0') {          // this+0x6DF: C4-planted flag
    iVar3 = *(int *)&this->field_0x530;   // this+0x530: C4 timer duration
    if (*(int *)&this->field_0x528 == -1) { // this+0x528: C4 timer start frame
LAB_004401fe:
        if (iVar3 != 0) goto LAB_00440378;  // timer not started or non-zero → skip dispatch
    } else {
        iVar12 = g_CurrentFrameCounter - *(int *)&this->field_0x528;
        if (iVar12 < iVar3) {
            iVar3 = iVar3 - iVar12;
            goto LAB_004401fe;              // timer not elapsed yet → skip
        }
    }
    // Timer elapsed (or duration was 0):

    iStack_28 = this->Health;
    if (this->Type[0x16b6] == '\0') {       // BuildingTypeClass+0x16B6: IsBridgeRepairHut?
        // Not a BridgeRepairHut → generic C4 damage path (not in scope)
        (**(code **)(this->vtable + 0x16c))(
            &iStack_28, 0,
            *(undefined4 *)(g_RulesClass_Instance + 0xfa8),
            *(undefined4 *)&this->field_0x540,
            1, 0, 0);
    } else {
        // BridgeRepairHut: scan 5x5 neighbourhood for bridge tile type
        iVar3 = -2;
        do {
            iVar12 = -2;
            do {
                // Get cell at (hut_cell.X + iVar12, hut_cell.Y + iVar3)
                psVar6 = (short *)(**(code **)(this->vtable + 0x1b8))(auStack_24);
                uStack_3c = CONCAT22(psVar6[1] + (short)iVar3, (short)iVar12 + *psVar6);
                uStack_34 = uStack_3c;
                iVar5 = MapClass__Get_CellClass(&uStack_34);     // cell overlay index
                iVar5 = *(int *)(iVar5 + 0x38);                  // CellClass+0x38: tile type index

                // Check CellClass+0x44 (overlay or bridge subtype)
                psVar6 = (short *)(**(code **)(this->vtable + 0x1b8))(auStack_24);
                uStack_34 = CONCAT22(psVar6[1] + (short)iVar3, (short)iVar12 + *psVar6);
                uStack_2c = uStack_34;
                iVar7 = MapClass__Get_CellClass(&uStack_2c);

                // Low-bridge tile: tile-type index within [DAT_00abad1c, DAT_00abad1c+0x10)
                // High-bridge overlay: CellClass+0x44 in range (0x49, 0x66) exclusive
                if (((DAT_00abad1c <= iVar5) && (iVar5 < DAT_00abad1c + 0x10)) ||
                    ((0x49 < *(int *)(iVar7 + 0x44) && (*(int *)(iVar7 + 0x44) < 0x66)))) {
                    uStack_3c = 0x1000000;  // sentinel: bridge tile found
                }
                iVar12 = iVar12 + 1;
            } while (iVar12 < 3);           // inner: columns -2 .. +2
            iVar3 = iVar3 + 1;
        } while (iVar3 < 3);                // outer: rows -2 .. +2

        // Dispatch based on whether a low-bridge tile was found
        if (uStack_3c._3_1_ == '\0') {
            // High-bridge path (no low tile found in 5x5 scan)
            uVar8 = (**(code **)(this->vtable + 0x1b8))(auStack_20);
            MapClass__DestroyBridge_High_OnHutDeath(uVar8);   // 0x0044031B → 0x00574000
        } else {
            // Low-bridge path (low-bridge tile found)
            uVar8 = (**(code **)(this->vtable + 0x1b8))(auStack_20);
            MapClass__DestroyBridge_Low_OnHutDeath(uVar8);    // 0x00440301 → 0x00574C20
        }
        this->field_0x6df = 0;       // 0x00440320: clear C4-planted flag (+0x6DF)
        *(undefined4 *)&this->field_0x540 = 0;  // 0x00440327: clear stored attacker (+0x540)
    }
    // ... sell-on-capture check and vtable+0x124 call follow if this->field_0x90
}
```

## Behavioral Analysis

### Entry condition

The BridgeRepairHut branch is reached when ALL of the following hold (in order):

1. `BuildingClass::Update` is called for this building (every game tick).
2. `this->Health > 0` (building is alive; dead path exits earlier at `LAB_004400c1`).
3. `this->field_0x6DF != 0` — C4 charge is planted on this hut.
4. Timer has elapsed: `this->field_0x528 != -1` AND
   `g_CurrentFrameCounter - this->field_0x528 >= this->field_0x530`.
5. `BuildingTypeClass+0x16B6 != 0` — this building type is a BridgeRepairHut.

If conditions 3–4 are met but condition 5 is false, the generic C4 damage vtable call
fires instead (out of scope for this decode).

### 5×5 scan — bridge type detection

The scan iterates `iVar3` and `iVar12` each from -2 to +2 inclusive (25 cells). For each
cell it calls `this->vtable+0x1b8` (GetCellPacked / NW cell), adds the delta pair to
get the target cell coordinates, then calls `MapClass::Get_CellClass` to retrieve the cell.

Two independent checks classify the cell as a bridge cell:

| Check | Field | Condition | Bridge type |
|---|---|---|---|
| Tile-type table | `CellClass+0x38` (tile type index) | `DAT_00abad1c <= val < DAT_00abad1c + 0x10` | Low bridge |
| Overlay subtype | `CellClass+0x44` (overlay/subtype) | `0x49 < val < 0x66` (i.e. 0x4A–0x65) | High bridge |

`DAT_00abad1c` is a runtime-populated tile-type table base (static value zero; populated at
game-load). Verified present via `get_xrefs_to 0x00abad1c`: referenced by
`CellClass__IsWoodBridge`, `ProcessBridgeDamageStateMachine_Low`, and
`Apply_area_damage`.

If either check fires for any cell in the 5×5 neighbourhood, `uStack_3c` is set to
`0x01000000` (byte 3 of the local = non-zero sentinel).

### Dispatch

After the 5×5 scan, `uStack_3c._3_1_` (byte 3 of the packed sentinel) determines dispatch:

| `uStack_3c` byte 3 | Path | Assembly address | Target |
|---|---|---|---|
| `0` (no low-tile found) | High bridge | `0x0044031B` | `MapClass::DestroyBridge_High_OnHutDeath` @ `0x00574000` |
| non-zero (low-tile found) | Low bridge | `0x00440301` | `MapClass::DestroyBridge_Low_OnHutDeath` @ `0x00574C20` |

Dispatch addresses verified via `get_xrefs_from 0x00440301` and
`get_xrefs_from 0x0044031B`.

Both dispatch targets are also called from `BombClass::Detonate` (verified via
`get_xrefs_to 0x00574C20` and `get_xrefs_to 0x00574000`), indicating the same
destroy-bridge logic is shared between C4-on-hut and direct bomb detonation paths.

### Post-dispatch cleanup

Immediately after dispatch (regardless of which path was taken):

- `this->field_0x6DF = 0` — clears the C4-planted flag (assembly `0x00440320`)
- `*(uint32*)&this->field_0x540 = 0` — clears stored attacker / C4 warhead owner
  (assembly `0x00440327`)

These are simple store instructions; no outbound refs expected from those addresses
(confirmed: `get_xrefs_from 0x00440320` and `get_xrefs_from 0x00440327` returned
no references).

### Post-dispatch conditional sell / close

After the cleanup block, if `this->field_0x90 != 0`, the code calls
`this->vtable+0x124` with argument `2` (a "close gate" or "destroy building" command,
out of scope). This is the normal building-death path.

## Struct Field Accesses

All offsets from `this` (BuildingClass instance pointer; param is `BuildingClass *this`
— direct byte offsets, not `int *` multiplication needed).

| Offset | Size | Field | Role in bridge branch |
|---|---|---|---|
| `+0x6DF` | 1 byte | C4-planted flag | Non-zero → C4 is armed on this hut; zeroed at `0x00440320` post-dispatch |
| `+0x528` | 4 bytes | C4 timer start frame | `g_CurrentFrameCounter` value when C4 was planted; `-1` = not started |
| `+0x530` | 4 bytes | C4 timer duration | Number of frames to wait after `+0x528` before detonation |
| `+0x540` | 4 bytes | C4 attacker slot | Stored attacker reference (used for kill-credit); zeroed post-dispatch |
| `Type` ptr | ptr | BuildingTypeClass* | Base for type field lookups |
| `Type+0x16B6` | 1 byte | IsBridgeRepairHut flag | Selects bridge-destroy path vs generic C4 damage |

Coordinate reference frame: `this->vtable+0x1b8` (GetCellPacked) returns NW-corner cell
index (cells, not leptons). The 5×5 scan deltas are applied in cell-space.
`CellClass+0x38` is the tile-type index field; `CellClass+0x44` is the overlay/subtype
field (both from `MapClass::Get_CellClass` return value, not from BuildingClass itself).

## Globals Referenced

| Global | Role |
|---|---|
| `g_CurrentFrameCounter` | Frame counter for timer expiry comparison |
| `DAT_00abad1c` | Tile-type table base (low-bridge tile indices); runtime-populated |
| `g_RulesClass_Instance + 0xfa8` | C4 warhead (used in non-hut generic path; not in scope) |

## Out-of-scope Refs

Symbols seen in or adjacent to the branch but not in current decode scope:

- `MapClass::DestroyBridge_Low_OnHutDeath` @ `0x00574C20` — decode task #3
- `MapClass::DestroyBridge_High_OnHutDeath` @ `0x00574000` — decode task #4
- `BombClass::Detonate` @ `0x00438960` — also calls both destroy-bridge functions;
  shared logic but not in current scope
- `vtable+0x16C` call — generic C4 damage path (non-hut branch); out of scope
- `ProcessBridgeDamageStateMachine_Low` — reads `DAT_00abad1c`; bridge damage state
  machine, referenced by tile-type table xrefs
- `CellClass__IsWoodBridge` @ `0x00486770` — reads `DAT_00abad1c`; bridge tile type
  classification helper
- `InfantryClass::PerCellProcess` — sets `this+0x6DF`, arms `+0x528`/`+0x530` timer;
  decode task #2 (in progress)

## Unverified Claims (YELLOW)

- The exact field meaning of `CellClass+0x44` as "overlay subtype" is inferred from the
  range check `0x49 < val < 0x66` and the context (high-bridge detection). The field name
  is not confirmed from a struct layout decode; `decode-struct-CellClass_BridgeFields`
  (task #21) will verify.
- `DAT_00abad1c` is asserted to be a tile-type table populated at game-load. Static read
  at analysis time returned all zeros (expected for runtime data). The `CellClass__IsWoodBridge`
  and `ProcessBridgeDamageStateMachine_Low` xref context supports this interpretation but
  no decompile of those functions was performed in this decode.
- The vtable slot `+0x1b8` is identified as GetCellPacked (returns NW-corner cell index)
  per CLAUDE.md coordinate conventions table. Not independently re-verified via
  `decompile_function` on the slot target in this decode; cross-check expected from
  `decode-struct-BuildingTypeClass_BridgeFields` (task #19).
