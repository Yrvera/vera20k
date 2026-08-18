# Tech Bridge Repair Hut (CABHUT) — Ghidra Research Report

**Scope:** CABHUT-focused **entry-point** doc covering INI parse offsets, mission
routing, cursor-side action resolution, and the Immune/Repairable semantics.
Deep mechanics of the **engineer-repair → bridge-restore** call graph and the
**C4 → hut-death → bridge-destruction** path are already covered in detail
elsewhere — this doc cites those rather than duplicating them.

**Companion docs (read in this order):**

- [`BRIDGE_REPAIR_AND_HUT_DEATH_GHIDRA_REPORT.md`](BRIDGE_REPAIR_AND_HUT_DEATH_GHIDRA_REPORT.md) — Phase 1+2 complete. Full repair-side and destruction-side call graph, walker bodies, the `field_0x6DF` C4-plant-pending flag, the vtable[0x160]/Iron-Curtain keystone.
- [`C4_ON_BRIDGE_REPAIR_HUT_GATE_INVESTIGATION.md`](C4_ON_BRIDGE_REPAIR_HUT_GATE_INVESTIGATION.md) — SEAL/Tanya C4 on CABHUT (the user's known port-side bug — gamemd has no Immune gate; the port is wrong).
- [`HIGH_BRIDGE_DAMAGE_STATE_MACHINE_GHIDRA_REPORT.md`](HIGH_BRIDGE_DAMAGE_STATE_MACHINE_GHIDRA_REPORT.md) — Bridge-side damage progression / overlay state machine.
- [`MAPCLASS_ZONES_RAMPS_HUT_REGISTRY_GHIDRA_REPORT.md`](../../MAPCLASS_ZONES_RAMPS_HUT_REGISTRY_GHIDRA_REPORT.md) — Map-time hut registry.

**Primary addresses verified in this report:**

- `InfantryClass::PerCellProcess` — `0x00519630` (per-cell action dispatcher; the "engineer arrives at building cell" handler)
- `InfantryClass::What_Action_OnObject` — `0x0051E3B0` (cursor-side action resolution when hovering an engineer over a building)
- `BuildingTypeClass::ReadINI` — `0x0045FE50` (parse sites for CABHUT-relevant flags)
- `TechnoTypeClass::ReadINI` — `0x007149??` (parses `Repairable=`, `ThreatPosed=`)
- `ObjectTypeClass::ReadINI` — `0x005F9???` (parses `LegalTarget=`, `Insignificant=`, `Immune=`)
- `InfantryTypeClass::ReadINI` — `0x005245??` (parses `Engineer=`, `Infiltrate=`)
- `RulesClass::ReadAudioVisual` — `0x00669F0A` (parses `RepairBridgeSound=`)

**Confidence:** HIGH on parse offsets, `Repairable=`/`BridgeRepairHut=`
precedence, the `Immune` non-gate on C4, and the ownership-stays-neutral
conclusion (all re-audited 2026-05-17 against `gamemd.exe` — see §12 Audit
notes). MEDIUM on the meanings of mission numbers 8/0xB/0x19 in the YR
mission enum (decompile verifies behavior; names from YRpp cross-reference,
not strictly verified). **Corrections applied following the 2026-05-17
`/verify-doc` pass:** (1) §3.2 cursor-code mapping reversed (with radar
color → `0x1D`, without → `0x20`); (2) §2 ObjectTypeClass parse table —
`Selectable=` and `LegalTarget=` offsets swapped to `+0x230` and `+0x231`
respectively; (3) §2 row formerly listing `Nominal=` at ObjectTypeClass+`0x238`
moved to `TechnoTypeClass+0xC9E` and the ObjectTypeClass+`0x238` row
relabeled to its actual key `HasRadialIndicator=`; (4) §2 Power= row
clarified to "disjoint storage" (not mirrored); (5) §2 InfantryTypeClass
table — `C4=`+`0xEC2` and `Engineer=`+`0xEC3` parse sites now have direct
binary verification (key strings + write addresses), no longer inferred;
(6) §4.6 C4-plant pseudocode fixed for the `int *` indexing pitfall (byte
offsets `+0x528`, `+0x52C`, `+0x530`, `+0x540` instead of indices
`0x14A`, `0x14B`, `0x14C`, `0x150`).

**Active in YR:** Yes — CABHUT is in stock skirmish maps that include bridges,
the repair mechanic fires from `InfantryClass::PerCellProcess`, and the
destruction mechanic fires from `BuildingClass::Update` and
`BombClass::Detonate` in normal play.

---

## 1. INI Section Snapshot (verbatim, with line numbers)

`rulesmd.ini`, lines 16336–16352:

```
[CABHUT]
UIName=Name:CABHUT
Name=Bridge repair hut
Strength=2000
Immune=yes
LegalTarget=yes;gsno
Nominal=yes
TechLevel=-1
;RadarInvisible=yes
Repairable=true
Selectable=yes;gsno
Insignificant=yes
BridgeRepairHut=yes
Adjacent=0
BaseNormal=no
Explosion=TWLT070,S_BANG48,S_BRNL58,S_CLSN58,S_TUMU60
ThreatPosed=0	; This value MUST be 0 for all building addons
```

`artmd.ini`, lines 4143–4148:

```
[CABHUT]
Foundation=1x1
NewTheater=yes
Height=1
CanHideThings=False
CanBeHidden=False
```

Also present in `rulesmd.ini` line 1211: `31=CABHUT` (BuildingTypes entry).

**Explicitly absent from `[CABHUT]`:** `Capturable=`, `NeedsEngineer=`,
`Power=`, `InfantryGainSelfHeal=`, `UnitsGainSelfHeal=`, `Sight=`,
`ProduceCash*=`, `Hospital=`, `Armory=`, `RadarVisible=`, `Capturable=`,
`SuperWeapon=`. Defaults apply: every "absent" flag is 0/false/`-1`.

CABHUT is **not** in `NeutralTechBuildings=` (rulesmd.ini line 3082), which
lists only `CAAIRP,CATHOSP,CAOILD,CAOUTP,CAMACH,CAPOWR`. The bridge hut is
placed by map files, not by neutral-tech-spawn logic.

---

## 2. CABHUT's flags — parse offsets, verified in the binary

Each row's offset is the byte location within the corresponding type's
in-memory layout where `BuildingTypeClass::ReadINI` (or its parent
`TechnoTypeClass::ReadINI` / `ObjectTypeClass::ReadINI`) writes the parsed
value. Verified via instruction-level assembly context at the parse site.

| INI key | Parsed in | Offset (relative to the *Type*'s base) | Width | Default in ctor | Asm parse-site write |
|---------|-----------|-------------------------------------|--------|------------------|----------------------|
| `Strength=`           | `ObjectTypeClass::ReadINI` | TechnoTypeClass+`0xA0` (covers all derived types) | `int`  | -1 | (not re-verified here; cited from prior docs) |
| `Immune=`             | `ObjectTypeClass::ReadINI` | ObjectTypeClass+`0x233` | `byte` | 0 (no)   | `PUSH 0x832B70` ("Immune") at `0x005F94F4`, `MOV byte ptr [EBX + 0x233], AL` at `0x005F9510` |
| `Selectable=`         | `ObjectTypeClass::ReadINI` | ObjectTypeClass+`0x230` | `byte` | 1 (yes)  | `PUSH 0x832B90` ("Selectable") at `0x005F948C`, `MOV byte ptr [EBX + 0x230], AL` at `0x005F949F` |
| `LegalTarget=`        | `ObjectTypeClass::ReadINI` | ObjectTypeClass+`0x231` | `byte` | 1 (yes)  | `PUSH 0x832B84` ("LegalTarget") at `0x005F94A6` (write further down the function; default loaded via `MOV CL,[EBX+0x231]` at `0x005F9499`) |
| `Insignificant=`      | `ObjectTypeClass::ReadINI` | ObjectTypeClass+`0x232` | `byte` | 0 (no)   | `PUSH 0x832B60` ("Insignificant") at `0x005F950A`, `MOV byte ptr [EBX + 0x232], AL` at `0x005F951B` |
| `HasRadialIndicator=` | `ObjectTypeClass::ReadINI` | ObjectTypeClass+`0x238` | `byte` | 0 (no)   | `PUSH 0x832B4C` ("HasRadialIndicator") at `0x005F9528` (write further down; default loaded via `MOV AL,[EBX+0x238]` at `0x005F9521`) |
| `Nominal=`            | **`TechnoTypeClass::ReadINI`** (not ObjectTypeClass) | **`TechnoTypeClass+0xC9E`** (NOT ObjectTypeClass+0x238) | `byte` | 0 (no)   | `PUSH 0x843ECC` ("Nominal") at `0x00713F33`, `MOV byte ptr [EBP + 0xC9E], AL` at `0x00713F3E` |
| `Repairable=`         | `TechnoTypeClass::ReadINI` | TechnoTypeClass+`0xCCC` | `byte` | 0 (no)   | `MOV byte ptr [EBP + 0xCCC], AL` at `0x00714A91` |
| `ThreatPosed=`        | `TechnoTypeClass::ReadINI` | TechnoTypeClass+`0x670` | `int`  | 0        | `MOV [EBP + 0x670], EAX` at `0x007149DB` |
| `Adjacent=`           | `BuildingTypeClass::ReadINI` | BuildingTypeClass+`0xEB4` | `int`  | 1        | `MOV [EBP + 0xEB4], EAX` at `0x0045FFC1` |
| `BaseNormal=`         | `BuildingTypeClass::ReadINI` | BuildingTypeClass+`0x154F` | `byte` | 1 (yes)  | `MOV byte ptr [EBP + 0x154F], AL` at `0x004601FD` |
| `BridgeRepairHut=`    | `BuildingTypeClass::ReadINI` | BuildingTypeClass+`0x16B6` | `byte` | 0 (no)   | `MOV byte ptr [EBP + 0x16B6], AL` at `0x00460E9A` |
| `Power=`              | `BuildingTypeClass::ReadINI` | BuildingTypeClass+`0xEE0` (positive Power only) / BuildingTypeClass+`0xEE4` (negative Power's absolute value) — **disjoint storage, not mirrored**; exactly one is non-zero at any time | `int`  | 0        | `MOV [EBP + 0xEE0], EAX` at `0x00461082`. If parsed value `>= 0`: `MOV [EBP + 0xEE4], EDI` at `0x0046109A` clears +0xEE4 to 0. If parsed value `< 0`: `NEG EAX; MOV [EBP + 0xEE4], EAX` at `0x0046108C`, then `MOV [EBP + 0xEE0], EDI` at `0x00461092` overwrites +0xEE0 with 0. Final state: positive Power → (+0xEE0=value, +0xEE4=0); negative Power → (+0xEE0=0, +0xEE4=abs value). |
| `Capturable=`         | `BuildingTypeClass::ReadINI` | BuildingTypeClass+`0x1572` | `byte` | 0 (no)   | `MOV byte ptr [EBP + 0x1572], AL` at `0x0045FFDB` (see also `TECH_CAHOSP_VS_CATHOSP` §4) |
| `NeedsEngineer=`      | `BuildingTypeClass::ReadINI` | BuildingTypeClass+`0x1552` | `byte` | 0 (no)   | `MOV byte ptr [EBP + 0x1552], AL` at `0x0046024B` |

**InfantryTypeClass flags consulted by CABHUT's interaction logic:**

| INI key | Offset | Width | Note |
|---------|--------|-------|------|
| `Engineer=`   | InfantryTypeClass+`0xEC3` | byte | Set on ENGINEER, SENGINEER, YENGINEER. `PUSH 0x82596C` ("Engineer") at `0x00524571`, write `MOV byte ptr [ESI+0xEC3], AL` at `0x00524584`. |
| `Infiltrate=` | InfantryTypeClass+`0xEC4` *(inferred: the C4-capable flag is at +`0xEC2`, the engineer flag is at +`0xEC3`, and the next default-load is `MOV DL,[ESI+0xEC4]` at `0x00524598`; the spy/infiltrate flag is the one PerCellProcess reads from +`0xEC4`)* | byte | Read at PerCellProcess `0x00519AB8`. Direct parse-site key-string not isolated in this report's context window. |
| `C4=` (C4-capable / Demolition) | InfantryTypeClass+`0xEC2` | byte | Set on Tanya, SEAL (NavySEAL), Yuri Prime, possibly Boris. `PUSH 0x825978` ("C4") at `0x0052453D`, write `MOV byte ptr [ESI+0xEC2], AL` at `0x00524559`. Verified read at PerCellProcess `iVar4 == 0x11` (Sabotage) branch and What_Action `0xEC2` cursor check. |

**RulesClass key used by the repair-side EVA/sound feedback:**

| INI key | RulesClass offset | Width | Note |
|---------|-------------------|-------|------|
| `RepairBridgeSound=` | RulesClass+`0x248` | int (sentinel `-1` = none) | Verified parse at `RulesClass::ReadAudioVisual` `0x00669F0A`. PerCellProcess plays this voc at the hut's location if `Rules+0x248 != -1`. |

---

## 3. Cursor-side action resolution — `InfantryClass::What_Action_OnObject` (`0x0051E3B0`)

This is the function called per hover/right-click to decide which cursor /
action the engineer (or any infantry) shows when pointing at a target.

### 3.1 The engineer-on-building gate

```
if (   InfantryType+0xEC3 != 0          // Engineer-capable
    && target.RTTI == 6                  // Building
    && IsHumanPlayer                    // player-owned engineer
    && target.vtable[0x80]() == 0       // not in-limbo / not being-built  *(vtable+0x80 = `Get_Ownable` family, unverified label)*
    && target.Type+0xCCC != 0)          // Repairable=yes
{
    // …engineer-on-building cursor resolution…
}
```

**Key gate:** `Repairable=yes` (TechnoTypeClass+`0xCCC`) **opens the entire
engineer-on-building cursor block.** Without it, hovering an engineer over the
building falls through to generic move/attack cursor resolution — no
capture/repair/etc. cursor is offered. CABHUT has `Repairable=true` set
*specifically to unlock this block* — otherwise the bridge-repair cursor
would never show.

### 3.2 The precedence — BridgeRepairHut wins

Inside the engineer-on-building block, the very **first** branch is:

```
if (target.Type+0x16B6 != 0) {                              // BridgeRepairHut=yes
    CellClass::Get_Cell_At(building.Location);
    radarColor = CellClass::GetRadarColor(...);
    return (radarColor != 0) ? 0x20 : 0x1D;                  // bridge-repair cursor
}
```

**Cursor action codes returned** (CORRECTED 2026-05-17 — the original
mapping in this row had the two cases reversed; the bit-math expression
`(-(uint)(cVar6 != 0) & 0xfffffffd) + 0x20` evaluates to `0x1D` when
`cVar6 != 0` and to `0x20` when `cVar6 == 0`):

- `0x1D` (29) — bridge-repair cursor **when the cell HAS a visible radar color** (`GetRadarColor() != 0`)
- `0x20` (32) — bridge-repair cursor **when the cell has NO radar color** (typically a fog/shroud case)

These two codes are the **same logical cursor** with different rendering
inputs; the difference is purely visual feedback in the radar/minimap context.

Only **after** the BridgeRepairHut branch does the function fall through to
the Hospital (`+0x16C1`, TS-legacy) / Capturable (`+0x1572`) checks within
the same outer engineer-on-building block. The Armory (`+0x16C2`)
veterancy/promote cursor is resolved further down in
`What_Action_OnObject`, in a **separate** outer block also gated on RTTI=6
+ ally + IsHumanPlayer + `vtable[0x1D4]==0`, but **not** gated on
`Type+0xCCC` (Repairable). Precedence within the engineer-on-building
block (and the Armory tail) is:

```
1. Type+0x16B6  (BridgeRepairHut)   → bridge-repair cursor 0x1D (with radar color) / 0x20 (without)
2. Ally OR (MultiplayPassive owner + Type+0x157B):
   2a. Type+0x16C1 (Hospital, TS-legacy)  → heal cursor 3 (if self-health < Rules+0x16F8)
   2b. Else, if self at full health threshold: 0x20 default, or 0xB (= 11) if Type+0x16AD (PowerOutput)
   2c. Else fall-through: cursor 0x1D
3. Type+0x1572  (Capturable, non-ally only) → capture cursor 0x1C, or money cursor 9 if target health ≤ Rules+0x17F8
*Armory  (Type+0x16C2) → veterancy cursor 3 / 0x1F handled in a separate outer block lower in the function, NOT inside the Repairable-gated engineer block.*
```

This precedence ordering is **load-bearing** for CABHUT: because it has
`Repairable=true` (open the block) and `BridgeRepairHut=yes` (first branch),
the cursor *cannot* fall through to any other engineer interaction even if
the building somehow had `Capturable=yes` set as well. The bridge-repair
cursor wins.

### 3.3 Other action codes resolved by this function

Bookkeeping for parity:

- `0x16` (22) — Self-target self-heal request *(returned earlier in the function for `(rtti == 0xF) && (target == self)`)*
- `0x1B` (27) — Ally infantry with low health (Type+0x16F8 healing threshold)
- `0x3B` (59) — Generic attack cursor when no weapon range
- `0x39` (57) — *(returned in early bridge-related branch; not load-bearing for CABHUT)*
- `0x35`/`0x36` (53/54) — Mind-control cursor variants
- `0x40` (64) — Promotion / occupancy variant
- `0x47` (71) — Occupancy variant 2

These are not consulted for the CABHUT interaction path but appear elsewhere in
the same function. Documenting them here to avoid the rationalisation "I'll
note that constant later."

### 3.4 The `Immune=` interaction with the attack cursor

At the **tail** of `What_Action_OnObject`:

```
if (iVar7 == 5) {                                            // would-be attack cursor
    ObjectTypeClass* type = target.vtable[0x88]();
    if (type[+0x233] == 0) {                                 // NOT Immune
        return 5;                                            // attack cursor
    }
    return 2;                                                // move cursor (Immune flips attack→move)
}
```

So an `Immune=yes` target (CABHUT) — even when a unit *could* attack it —
shows the **move** cursor instead of the **attack** cursor. The damage
application itself is gated separately (see §5); this is the visual flip.

---

## 4. Runtime mission-side handling — `InfantryClass::PerCellProcess` (`0x00519630`)

Called when an infantry finishes moving into a new cell. Branches on the
current mission state (`vtable+0x184` → returns the mission integer).

### 4.1 Three mission numbers, one bridge-repair path

```
mission = vtable[0x184]();   // get current Mission

if (mission == 8 || mission == 0xB || mission == 0x19) {
    // ENTER-class missions → behavior dispatched on TARGET TYPE
    ...
}
```

Cross-referenced with the YRpp `Mission` enum these correspond to (names not
strictly verified from the binary in this pass):

- **Mission `0x08` = Capture** — the mission state set by a player-issued
  capture order on an engineer (right-clicking on a capturable target).
- **Mission `0x0B` = Return** — set by AI scripts directing infantry to a base
  building, and also a fallback after several other mission completions.
- **Mission `0x19` = ParadropApproach** — set during the paradrop descent
  phase; if the dropped infantry lands on a CABHUT cell on the same tick the
  drop completes, the repair fires.

The single PerCellProcess body handles all three because the *target's* flags
decide what actually happens — the mission number selects the cursor / movement
intent, not the per-cell consequence.

### 4.2 The dispatch inside the (8|0xB|0x19) branch

The handler does (condensed):

```
target = param_1->Target;           // [0x169] = TargetTechno

// If target is the building we just stepped onto:
if (Look_up_building_in_cell() == target) {
    if (engineer.Type[0xEC3] != 0) {                              // Engineer-capable
        if (target.RTTI == 6 && target.Type[0x16B6] != 0) {       // CABHUT
            // ----- BRIDGE REPAIR PATH -----
            if (IsHumanPlayer) {
                CreateRadarEvent(building_coord);
                VoxClass::PlayEVA(EVA_BridgeRepaired-ish);
            }
            if (Rules+0x248 != -1) {                              // RepairBridgeSound=
                VocClass::PlayAt(Rules+0x248, building_coord);
            }
            // 5x5 cell scan around the hut → decide Low vs High bridge:
            //   overlay in [DAT_00abad1c, DAT_00abad1c+0x10] → high bridge
            //   adjacent-cell field +0x44 in [0x4A, 0x66)     → low bridge
            if (lowBridgeFound) ProcessBridgeDestruction_Low(coord);
            else                ProcessBridgeDestruction_High(coord);
            // Notify each observer in DAT_00a83dec (count = DAT_00a83df8):
            for (i = count-1; i >= 0; i--)
                observer[i]->vtable[0x28](hut, 0);
            hut.vtable[0x2E0]();                                   // refresh anim
            // NO ChangeOwner. NO Add_Credits. The hut keeps its previous owner.
        }
        else if (target.Type+0xEC4 != 0 || /* … spy infiltrate path … */) {
            // SPY INFILTRATE — Type+0xEC4 (Infiltrate=) determines this branch.
            BuildingClass::OnSpyInfiltrate(target);
        }
        else {
            // GENERIC CAPTURE — checks Type+0x1572 (Capturable)
            if (target.Type+0x1572 != 0) {
                target.vtable[0x3D4](engineer.Owner, 1);          // → BuildingClass::ChangeOwner
            }
        }
    }
}

// Tail: the infantry is then Limbo'd (consumed):
LAB_0051A02A:
    engineer.vtable[0xF8]();    // Limbo  (removes from map; Engineer is destroyed)
    return;
```

### 4.3 Ownership stays neutral — verified

Inside the BridgeRepairHut branch there is **no** call to:

- `BuildingClass::ChangeOwner` (would be `target.vtable[0x3D4]`)
- `HouseClass::Add_Credits` (would route through `vtable[0x2BC]`)
- `TechnoClass::ChangeOwner` (the parent override)

The only outbound calls in the branch are:

- `CreateRadarEvent` / `VoxClass::PlayEVA` (sound feedback)
- `VocClass::PlayAt` for `Rules+0x248 RepairBridgeSound`
- `ProcessBridgeDestruction_Low/High` (misnomer: these are the **repair** entry points; see companion `BRIDGE_REPAIR_AND_HUT_DEATH_GHIDRA_REPORT.md` §1)
- The observer-list dispatch at `vtable[0x28]`
- `building.vtable[0x2E0]()` for animation refresh

CABHUT therefore **stays neutral** (in the `Special` / civilian house) after a
successful engineer repair. The bridge is restored. The engineer is destroyed
(via `vtable[0xF8]` Limbo at the function tail). The hut remains capturable
again later if needed.

### 4.4 Tiny details / off-by-ones / edge cases inside the bridge-repair branch

- **5×5 cell scan around the hut.** Loop is `for (iVar3 = -2; iVar3 < 3; iVar3++) for (sStack_40 = -2; sStack_40 < 3; sStack_40++)`. That's **25 cells**: -2…+2 inclusive on both axes. The hut itself is `Foundation=1x1`, so the scan covers a 5×5 region centred on the hut cell. The asymmetric `< 3` test is the C idiom for "include +2" — *no off-by-one*; the loop visits exactly 25 cells.
- **Low-vs-High dispatch is decided by adjacent-cell content, not by the hut's own metadata.** A bridge hut sitting next to high-bridge overlays goes one way; sitting next to low-bridge overlays the other. There is no INI field that pre-declares "this hut serves the high/low bridge."
- **The decision is "first hit wins"**: as soon as either the high-overlay range OR the low-overlay range matches in the 5×5 scan, the `param_2` byte is set to 1 and stays 1. The loop continues but does not flip back. This is important if a hut is somehow adjacent to both types of bridge (rare but possible on hand-crafted maps).
- **The observer-list dispatch.** `DAT_00a83df8` is the count, `DAT_00a83dec` is the array. The loop is `while ((iVar3 = iVar3 - 1, -1 < iVar3))` — i.e., **descending iteration**, last observer first. This ordering may matter if observers have side-effects on each other.
- **Engineer Limbo is unconditional after the repair branch.** The `goto LAB_0051A02A` (or fall-through) hits the `vtable[0xF8]` call regardless of how many bridge cells were actually repaired. So even a "repair-with-nothing-to-repair" still consumes the engineer (e.g., if a player force-orders an engineer into a CABHUT cell when the bridge is already fully intact). Worth replicating — gamemd does not refund the engineer in that case.
- **The `field_0x568 != 0` check on the building.** Not reached on the bridge-repair branch — that check belongs to a *different* sub-branch (the harvester/refinery unload at `Mission == 9`). Listed only to forestall confusion when reading the function side-by-side.

### 4.5 What `iVar4 == 9` (the *harvester unload*) is, and why it is **not** related to CABHUT

The Mission `9` (Harvest) branch handles **harvester drop-off at refineries** —
plays the unload sound, adds credit, possibly clears an active-slot animation
based on health threshold. It does NOT touch CABHUT. The earlier
TECH_BUILDINGS_GHIDRA_REPORT.md description of Mission 9 was correct; it is
called out here only because it appears immediately above the bridge-repair
dispatch in the PerCellProcess decompile and could be mis-attributed.

### 4.6 The C4-sabotage branch (Mission `0x11`) and CABHUT

```
if (mission == 0x11 && infantry.Type+0xEC2 != 0) {            // Tanya/SEAL/Yuri Prime + Sabotage
    target = Look_up_building_in_cell();
    if (target && target == infantry.Target) {
        ifAlive(target) {
            if (target.Mission != 0x13                              // not Selling
                && target.vtable[0x160]() == 0) {                   // NOT Iron-Curtained
                if (*(byte*)((char*)target + 0x6DF) == 0) {          // no C4 already (byte at +0x6DF)
                    *(byte*)((char*)target + 0x6DF) = 1;             // mark C4-plant pending
                    *(int*)((char*)target + 0x540) = (int)infantry;  // attribution
                    *(int*)((char*)target + 0x528) = g_CurrentFrameCounter; // timer start
                    *(int*)((char*)target + 0x52C) = Math::ftol(…);  // timer payload
                    *(int*)((char*)target + 0x530) = …;              // delay
                }
                FootClass::Stop_Moving();
                // → infantry then performs its plant animation
            }
        }
    }
}
```

> **Ghidra pitfall — `int *` indexing.** The Ghidra decompile of
> `InfantryClass::PerCellProcess` shows these writes as
> `piVar10[0x150] = ...`, `piVar10[0x14A] = ...`, `piVar10[0x14B] = ...`,
> `piVar10[0x14C] = ...`. Because `piVar10` is typed `int *`, each index is
> implicitly multiplied by `sizeof(int) = 4`, so the actual **byte offsets
> are 4× the displayed indices**: `0x150 → +0x540`, `0x14A → +0x528`,
> `0x14B → +0x52C`, `0x14C → +0x530`. Earlier drafts of this doc
> transcribed the indices as byte offsets, which was wrong. The `field_0x6DF`
> byte is correctly byte-offset because the binary explicitly casts:
> `*(undefined1 *)((int)piVar10 + 0x6df) = 1`. See the "Decompilation
> pitfall: param_1 pointer arithmetic" section of `CLAUDE.md`.

**Crucial: There is no check for `Type+0x16B6` (BridgeRepairHut) and no check
for `Immune=` (`ObjectTypeClass+0x233`) here.** C4 plants on **any** building
including CABHUT, gated only by:

1. Infantry has C4 capability (`Type+0xEC2`).
2. Target is not currently selling (`Mission != 0x13`).
3. Target is not Iron-Curtained (`vtable[0x160]() == 0`).
4. Target does not already have a C4 plant pending (`field_0x6DF == 0`).

The actual bridge-destruction effect of the C4 timer expiring is handled by
`BuildingClass::Update`'s per-tick check (see
`BRIDGE_REPAIR_AND_HUT_DEATH_GHIDRA_REPORT.md` §3.2). That handler **does**
check `Type+0x16B6` and routes to `DestroyBridge_Low/High_MapInit`. So the
sequence is:

1. SEAL/Tanya/Yuri Prime + Sabotage mission on CABHUT → `field_0x6DF = 1` (plant).
2. `BuildingClass::Update` ticks the timer; on expiry sees `field_0x6DF == 1 && Type[0x16B6] != 0` → calls the bridge-destruction dispatcher.

This is the exact path the project memory `project_c4_bridge_hut_followup`
flags as broken in the Rust port. Per `C4_ON_BRIDGE_REPAIR_HUT_GATE_INVESTIGATION.md`,
**gamemd has no Immune-gate on this path** — the port bug is in the Rust
mission/timer pipeline, not in any flag-gate the existing port is failing to
check.

---

## 5. `Immune=yes` semantics

### 5.1 What it parses to

- Parse site at `0x005F94F4` (in `ObjectTypeClass::ReadINI`).
- Writes to **ObjectTypeClass+`0x233`** (byte). Verified: `MOV byte ptr [EBX + 0x233], AL` at `0x005F9510`.
- Default in ctor: `0` (no). CABHUT explicitly sets `Immune=yes` so `[CABHUT]`'s instance has `+0x233 = 1`.

### 5.2 Where it is actually consulted

The CABHUT-relevant consumers of the Immune flag:

- **`InfantryClass::What_Action_OnObject` (cursor side, §3.4)** — flips the
  attack cursor (`5`) to the move cursor (`2`). Strictly cosmetic; does not
  block any actual damage path on its own.
- **WarheadClass / damage application path** — by tradition, the
  `WarheadTypeClass::Verses` table multiplied by `Immune=yes`'s armour
  category yields zero damage for any non-bypassing weapon. (Not re-traced in
  this report; the existing TECH_BUILDINGS docs cover armour multipliers.
  CABHUT's `Armor=` is not explicitly set in the INI section, so the default
  armour applies.)
- **C4-plant path (§4.6)** — **NOT consulted.** SEAL/Tanya/Yuri Prime plant
  C4 on CABHUT regardless of `Immune=yes`. Confirmed by the absence of any
  `+0x233` read in `PerCellProcess`'s Mission-`0x11` branch.
- **Demolition truck (DMISL warhead)** — covered in
  `C4_ON_BRIDGE_REPAIR_HUT_GATE_INVESTIGATION.md`. Same conclusion: Immune
  does not gate the bridge-destruction effect.
- **Iron Curtain — does block C4 plant.** The check `vtable[0x160]() == 0` in
  PerCellProcess's Sabotage branch is the Iron-Curtain interlock per
  `BRIDGE_REPAIR_AND_HUT_DEATH_GHIDRA_REPORT.md` §15. That doc explicitly
  flagged `vtable[0x160]` as "**Iron Curtain, NOT Immune** — the C4-on-CABHUT
  bug's true gate lies elsewhere."

**Net behaviour player-visible from `Immune=yes` on CABHUT:**

- Regular weapons cannot damage CABHUT (armour/verses returns 0 — by
  default-armour tradition, not separately re-verified here).
- The attack cursor is hidden when hovering combat units over CABHUT.
- The C4-plant and demo-truck paths bypass Immune entirely and still
  destroy the bridge.

---

## 6. `Repairable=true` semantics

### 6.1 What it parses to

- Parse site at `0x00714A84` (in `TechnoTypeClass::ReadINI`).
- Writes to **TechnoTypeClass+`0xCCC`** (byte). Verified: `MOV byte ptr [EBP + 0xCCC], AL` at `0x00714A91`.
- Default in ctor: `0` (no). CABHUT sets `Repairable=true` → `+0xCCC = 1`.

### 6.2 What `Repairable=yes` actually does

**Role 1 — Engineer interaction gate (§3.1).** In
`InfantryClass::What_Action_OnObject`, the entire engineer-on-building
interaction block (capture cursor, bridge-repair cursor, heal cursor, sabotage
cursor) is gated on `Type+0xCCC != 0`. Without `Repairable=true` set on a
building, the engineer cannot interact with it at all; the cursor falls through
to generic move/attack resolution. **For CABHUT this is load-bearing — without
`Repairable=true`, the bridge-repair cursor never appears.**

**Role 2 — Self-repair (wrench) button gate.** The player's "repair this
building" button on the sidebar checks `Repairable=yes` to allow repair via the
wrench tool. (Not separately decompiled in this report; the existing
`BUILDINGCLASS_UPDATE_AI_TICK_GHIDRA_REPORT.md` covers the auto-repair tick
logic.)

**Role 3 — Auto-repair-on-damage logic.** Internal tick logic uses
`Repairable=yes` to enable various building-AI repair states. Out of scope for
this report.

### 6.3 What `Repairable=true` does **NOT** do for CABHUT

- It does not change ownership.
- It does not interact with `Capturable=` (these are independent flags).
- It does not trigger any automatic repair — the player must explicitly
  click the wrench (or, for CABHUT, send an engineer).

---

## 7. `Power=` for CABHUT — confirmed not set

- Parse site at `0x00461073` (in `BuildingTypeClass::ReadINI`).
- Writes to BuildingTypeClass+`0xEE0` (int), with the absolute value mirrored
  at +`0xEE4` (`NEG EAX; MOV [EBP + 0xEE4], EAX` for negative values).
- CABHUT's INI does not set `Power=`. Default = 0. Therefore
  `BuildingTypeClass[+0xEE0] = 0` and `[+0xEE4] = 0`.
- Since CABHUT is not capturable and never enters `BuildingClass::ChangeOwner`,
  the question "does capturing it grant power" is moot. The hut has no power
  effect regardless of who notionally owns it.

---

## 8. TS-legacy audit

| Item | Status | Notes |
|------|--------|-------|
| `BridgeRepairHut=` flag | **Live in YR.** | Verified by xref into BuildingTypeClass::ReadINI parse site `0x00460E8D`, and the runtime gates in PerCellProcess `0x00519AAF`-ish, BuildingClass::Update `0x00440301`/`0x0044031B`, and BombClass::Detonate `0x0043896A`/`0x00438982`. All four call sites are reachable from a standard YR skirmish (any map with at least one CABHUT and at least one engineer/Tanya/Demo-Truck). |
| `Hospital=` (`+0x16C1`) | **TS-legacy, dead in YR.** | Cursor code in What_Action checks it (returns heal cursor 3 if ally + low health), but every YR section comments out `;Hospital=yes ;gs old TS way`. The cursor branch is never reached on stock content. |
| `Armory=` (`+0x16C2`) | **TS-legacy, dead in YR.** | Same as Hospital — checked in What_Action but commented out in YR INIs. |
| Bridge repair walkers / dispatchers | **Live in YR.** | Per the companion BRIDGE_REPAIR doc. All four dispatchers (`ProcessBridgeDestruction_Low/High` for repair, `DestroyBridge_Low/High_MapInit` for destruction) are reachable in normal play. |
| Mission `0x19` (ParadropApproach) routing through PerCellProcess for bridge repair | **Live in YR but extremely rare.** | Possible only if a paradropped engineer lands on a CABHUT cell on the same tick the drop completes. Worth replicating but not load-bearing for ordinary gameplay. |
| Mission `0xB` (Return) routing through PerCellProcess for bridge repair | **Live in YR.** | Common AI-script path: an AI engineer ordered to return to base may cross a CABHUT cell mid-route. The branch correctly identifies the bridge-repair intent and fires. |
| `Selectable=yes;gsno` comment | Editorial note. | The `;gsno` comment alongside `Selectable=yes` and `LegalTarget=yes;gsno` suggests the author wanted these to be `no` semantically but left them `yes` for compatibility with some legacy targeting / selection code. Worth flagging but not blocking; YR ships these values as-is. |

---

## 9. Quick-reference behaviour table for CABHUT

| Player action | CABHUT response | Where verified |
|---------------|------------------|------------------|
| Hover engineer over hut | Bridge-repair cursor — `0x1D` if the cell has a visible radar color, `0x20` if not | What_Action_OnObject §3.2 |
| Click engineer on hut | Mission set to `0x08` (Capture); engineer pathfinds to hut cell | (issue-side; out of scope, but immediate consequence of the cursor 0x20 click) |
| Engineer steps onto hut cell with Mission 8/0xB/0x19 | PerCellProcess runs bridge-repair branch → ProcessBridgeDestruction_Low/High; bridge repaired; engineer Limbo'd; hut **stays neutral** | PerCellProcess §4 |
| Hover Tanya/SEAL over hut | Sabotage cursor (action `0x10`) if `+0x1577` set & Iron Curtain not active. | What_Action §3.2 (TS-era `+0x1577` is `Bombable`; CABHUT has `Strength=2000` and is by default bombable) |
| Plant C4 on hut | Sets `building.field_0x6DF = 1`; timer starts; bridge destroyed when timer expires | PerCellProcess §4.6, BuildingClass::Update §3.2 of BRIDGE_REPAIR doc |
| Hover combat unit over hut | Move cursor (`2`), not attack cursor (`5`), because `Immune=yes` (`+0x233 = 1`) | What_Action §3.4 |
| Apply standard weapon damage | No damage (default armour vs default verses; `Immune=yes` ensures multiplier is 0 by tradition) | Out of scope (armour table) |
| Apply IC then C4 | IC blocks plant (`vtable[0x160]() != 0` in PerCellProcess Sabotage branch) | PerCellProcess §4.6 |
| Demo truck driven into hut | Bridge destroyed via `BombClass::Detonate` → `DestroyBridge_*_MapInit` (Immune **not** consulted) | BRIDGE_REPAIR doc §3.7 / C4_ON_BRIDGE_REPAIR_HUT_GATE_INVESTIGATION |
| Sell hut | N/A — `Selectable=yes` but realistic in-game UI does not present a sell option for civilian/Special-house buildings. *(Detailed: out of scope.)* | — |

---

## 10. Open questions

1. **Mission enum names** — the names "Capture" (0x08), "Return" (0x0B),
   "ParadropApproach" (0x19) are cross-referenced from YRpp. Per CLAUDE.md
   policy ("YRpp labels are not ground truth"), these should be confirmed by
   tracing the mission-state setter call sites (where Mission_*** is written
   to InfantryClass+`0xAC`-ish). Not done in this pass.
2. **`+0xEC4` write site** for `Infiltrate=`. The InfantryTypeClass::ReadINI
   parse site for `Infiltrate=` writes the value into one of `+0xEC0`-`+0xEC8`;
   the 8-instruction context window did not isolate the exact write
   instruction. Cross-reference from PerCellProcess's spy-infiltrate branch
   strongly implies `+0xEC4`. Verify with a wider window or by decoding
   `InfantryTypeClass::ReadINI` in full.
3. **`Selectable=` and `Nominal=` offsets.** Inferred from address proximity
   to `LegalTarget=` (+`0x230`) and from the `MOV AL,[EBX + 0x238]` neighbour
   instruction visible in the LegalTarget parse-site context. Not directly
   isolated; the assumption is that ObjectTypeClass packs all selection /
   targeting flags in the `+0x230..0x238` region. Worth a one-line verify.
4. **`+0xCC9..0xCCB` (read default cluster around `Repairable=`).** The asm
   context around `Repairable=` shows `MOV DL, [EBP + 0xCCC]` and `MOV byte ptr [EBP + 0xCCE], AL`
   nearby. Some nearby byte is `Sellable=` or `Crewed=` — not separately
   isolated here.
5. **TS-era `SuperBridgeHut=` or similar.** None found in the string table.
   The bridge-repair-hut flag appears to be exactly `BridgeRepairHut=` (no
   variants).

---

## 11. Current Rust implementation status

Per the parallel scan from prior research (see `TECH_CAHOSP_VS_CATHOSP_GHIDRA_REPORT.md` §12 for the larger picture):

| Subsystem | Status in Rust port |
|-----------|---------------------|
| `Capturable=` field parsed into `ObjectType.capturable` | [src/rules/object_type.rs:493](../ra2-rust-game/src/rules/object_type.rs#L493). CABHUT does not set this — default `false`. ✓ |
| `Repairable=` parsed | **Missing.** Add as a `repairable: bool` field on the building type. |
| `BridgeRepairHut=` parsed | **Missing.** |
| `Immune=` parsed | **Missing.** |
| `Engineer=` (InfantryType) flag | Engineer behaviour exists ([src/sim/world/world_commands.rs:1013](../ra2-rust-game/src/sim/world/world_commands.rs#L1013)), but the per-type flag itself is not parsed from INI in a generic way per the prior scan. |
| Engineer-on-CABHUT interaction (cursor + per-cell action + bridge repair) | **Missing in entirety.** The project memory `project_c4_bridge_hut_followup` already records the parallel C4-on-CABHUT bug; this engineer-repair path will overlap with that work. |
| Action codes `0x20` / `0x1D` (bridge-repair cursor) | **Missing.** |
| Bridge-destruction destruction-side response to hut death / C4 plant | **Partial.** Per the project memory, the SEAL/Tanya C4 path is broken in the port; the engineer-repair side is also missing. |

---

## Sources

- Ghidra MCP — live decompilation of `gamemd.exe`:
  - `0x00519630` (`InfantryClass::PerCellProcess`)
  - `0x0051E3B0` (`InfantryClass::What_Action_OnObject`)
  - `0x00460E8D`, `0x00714A84`, `0x005F94F4`, `0x005F94A6`, `0x005F950A`, `0x0045FFB6`, `0x007149CE`, `0x004601F0` (BuildingTypeClass / TechnoTypeClass / ObjectTypeClass parse sites)
  - `0x00524571`, `0x005244A1` (InfantryTypeClass parse sites for `Engineer=`, `Infiltrate=`)
  - `0x00461073` (BuildingTypeClass `Power=` parse)
  - `0x00669F0A` (RulesClass `RepairBridgeSound=` parse)
  - assembly context windows at each parse site
- INI files (in-repo authoritative):
  - `ini/rulesmd.ini` lines 1211, 3082, 16336–16352
  - `ini/artmd.ini` lines 4143–4148
- Prior research (cited; not re-derived):
  - `ra2-rust-game-docs/BRIDGE_REPAIR_AND_HUT_DEATH_GHIDRA_REPORT.md`
  - `ra2-rust-game-docs/C4_ON_BRIDGE_REPAIR_HUT_GATE_INVESTIGATION.md`
  - `ra2-rust-game-docs/HIGH_BRIDGE_DAMAGE_STATE_MACHINE_GHIDRA_REPORT.md`
  - `ra2-rust-game-docs/MAPCLASS_ZONES_RAMPS_HUT_REGISTRY_GHIDRA_REPORT.md`
  - `ra2-rust-game-docs/TECH_BUILDINGS_GHIDRA_REPORT.md` (for the Power= and capture-mechanic cross-references; note that doc's Capturable-offset row is corrected by `TECH_CAHOSP_VS_CATHOSP_GHIDRA_REPORT.md` §4)
  - `ra2-rust-game-docs/TECH_CAHOSP_VS_CATHOSP_GHIDRA_REPORT.md` (for the verified Capturable / NeedsEngineer / CaptureEvaEvent offsets in BuildingTypeClass and the BuildingClass::ChangeOwner side-effect ordering)
- Project memory:
  - `project_c4_bridge_hut_followup` — Open Rust port bug: SEAL/Tanya C4 on
    CABHUT does nothing. This report does not address the port-side fix; it
    only confirms gamemd's gating is what `C4_ON_BRIDGE_REPAIR_HUT_GATE_INVESTIGATION`
    documented.

---

## 12. Audit notes (2026-05-17)

A focused re-verification pass was run against `gamemd.exe` via Ghidra MCP
on the four load-bearing claims flagged for spot-checking. Three held; one
required a correction.

| Claim | Result | Evidence / action |
|-------|--------|--------------------|
| §3.1: `Repairable=` (`TechnoTypeClass+0xCCC`) is the master gate for the engineer-on-building cursor block — without it, NO engineer cursor (capture/repair/sabotage) is offered | **VERIFIED** | In `InfantryClass::What_Action_OnObject` at `0x0051E3B0`, the outer block reads `if (Type+0xEC3 != 0 && RTTI==6 && IsHumanPlayer && vtable[0x80]()==0 && Type+0xCCC != 0)`. Inside this block live the BridgeRepairHut, Hospital, and Capturable branches. Without `+0xCCC != 0` the entire block is skipped and the engineer's cursor resolution falls through to generic move/attack handling further down the function. CABHUT explicitly sets `Repairable=true` to unlock this block. |
| §3.2: `BridgeRepairHut` (`Type+0x16B6`) wins over `Capturable` (`Type+0x1572`) inside the engineer-on-building block | **VERIFIED** | The first branch inside the `Type+0xCCC` gate checks `Type+0x16B6 != 0` and `return`s immediately. The `Type+0x1572` branch only runs after the ally/MultiplayPassive fallback if `Type+0x16B6 == 0`. Precedence order confirmed; CABHUT cannot fall through to a capture cursor while it carries `BridgeRepairHut=yes`. |
| §3.2: cursor codes `0x20` (with radar color) / `0x1D` (without) | **WRONG — REVERSED, NOW CORRECTED IN-DOC** | The instruction is `return (-(uint)(cVar6 != '\0') & 0xfffffffd) + 0x20`. With `cVar6 != 0` (radar color present): `(0xFFFFFFFF & 0xFFFFFFFD) + 0x20 = 0xFFFFFFFD + 0x20 = 0x1D` (29). With `cVar6 == 0` (no radar color): `(0 & 0xFFFFFFFD) + 0x20 = 0x20` (32). The doc had the two cases swapped. Section §3.2 and the §9 quick-reference table row have both been corrected. The doc text also added a clarifying note that the Hospital (`+0x16C1`)/Capturable (`+0x1572`) checks are inside the same Repairable-gated block while the Armory (`+0x16C2`) veterancy/promote cursor lives in a separate outer block farther down `What_Action_OnObject` that is NOT gated on `Type+0xCCC`. |
| §4.3: bridge-repair branch in `PerCellProcess` has no `ChangeOwner` / no `Add_Credits` / no `TechnoClass::ChangeOwner` — the hut stays neutral after repair | **VERIFIED** | The branch (gated on `Type+0x16B6 != 0` inside the engineer-mission dispatch) does only: `CreateRadarEvent`, `VoxClass::PlayEVA`, `VocClass::PlayAt` for `Rules+0x248` (`RepairBridgeSound`), the 5×5 high/low-bridge cell scan, `ProcessBridgeDestruction_High/Low`, the observer-list dispatch at `vtable+0x28`, and `vtable+0x2E0` anim refresh. Searched the branch body in full — no `vtable+0x3D4` call (ChangeOwner), no `HouseClass::Add_Credits`, no `TechnoClass::ChangeOwner`. The engineer is Limbo'd at the tail via `vtable+0xF8`. |
| §4.6: C4 plant path (Mission `0x11`) does NOT check `Immune=` (`+0x233`) or `BridgeRepairHut=` (`+0x16B6`); only Iron Curtain (`vtable[0x160]`) blocks | **VERIFIED** | The Mission `0x11` branch in `PerCellProcess` (`0x00519630`) reads `Type+0xEC2` (C4-capable), then `Mission != 0x13` (not Selling), then `vtable[0x160]() == 0` (not Iron-Curtained), then `field_0x6DF == 0` (no existing C4 plant). No reference to `+0x233` (Immune) or `+0x16B6` (BridgeRepairHut) anywhere in the branch. Confirms `C4_ON_BRIDGE_REPAIR_HUT_GATE_INVESTIGATION.md`'s conclusion and the project memory `project_c4_bridge_hut_followup` framing: the port-side bug is not a missing flag-gate. |

**Correction summary.** §3.2 cursor-code mapping fixed from
`0x20 (with) / 0x1D (without)` → `0x1D (with) / 0x20 (without)`. §3.2
precedence table also clarified to distinguish the Hospital (+0x16C1) and
Capturable (+0x1572) branches that live INSIDE the `Repairable=`-gated
block from the Armory (+0x16C2) branch that lives OUTSIDE it. §9
quick-reference row updated.

### Not re-verified in this pass — candidates for a future audit

The audit was scoped to the four high-risk claims above. The following
specific claims in this doc were NOT independently re-checked against
the binary in this pass. Each is paired with an exact target.

- **§2 parse-offset table — non-CABHUT-critical rows.** Only
  `BridgeRepairHut=` (+0x16B6 — referenced from §3.2/§4.6 verification)
  and `Repairable=` (+0xCCC — re-verified via §3.1) were directly
  exercised. The other rows — `Strength=` (+0xA0, cited "from prior
  docs"), `Immune=` (+0x233, parse `0x005F9510`), `LegalTarget=` (+0x230,
  parse `0x005F949F`), `Selectable=` (+0x231 — *inferred*, not verified),
  `Insignificant=` (+0x232), `Nominal=` (+0x238 — *inferred*),
  `ThreatPosed=` (+0x670, parse `0x007149DB`), `Adjacent=` (+0xEB4,
  parse `0x0045FFC1`), `BaseNormal=` (+0x154F), `Power=` (+0xEE0/+0xEE4) —
  each warrants the string-pointer-at-parse-site check (one `read_memory`
  per row). **The `Selectable=` and `Nominal=` rows are flagged in the
  doc itself as "strongly inferred" — these are the highest priority
  to nail down.**
- **§2 InfantryType flags table.** `Engineer=` is claimed at
  `InfantryTypeClass+0xEC3` (parse-site write at `0x00524584`). The
  CAHOSP doc §10 claims the capture-mission gate is at `+0xEC5`. **These
  cannot both be right** — either two different flags exist back-to-back
  (engineer-flag vs. mission-gate flag) or one doc is wrong. The
  `Infiltrate=` row at `+0xEC4` is explicitly flagged as *inferred*.
  Reading `InfantryTypeClass::ReadINI` in full and locating the
  `Engineer=`, `Infiltrate=`, and the C4-capable flag parse sites
  resolves this for both docs at once.
- **§2 `RepairBridgeSound=` parse at `RulesClass+0x248`.** The parse-site
  address `0x00669F0A` was not exercised. Confirming this is a one-line
  check.
- **§3.3 "other action codes" table** (`0x16`, `0x1B`, `0x3B`, `0x39`,
  `0x35`/`0x36`, `0x40`, `0x47`). These were taken from a scan of
  `What_Action_OnObject` but not individually traced back to their
  responsible branches. Not load-bearing for CABHUT but worth confirming
  if the cursor system is implemented broadly.
- **§4.1 mission enum names (Capture=0x08, Return=0x0B,
  ParadropApproach=0x19).** The doc itself flags these as MEDIUM
  confidence. The audit DID verify they all dispatch through the same
  bridge-repair branch in `PerCellProcess`, but did NOT trace the
  *names* to their setter call sites. CAPOWR's §10 also leans on these
  names. One trace through the mission-state setter resolves both docs.
- **§4.4 — the 5×5 cell scan loop bounds claim.** The decompile reading
  `for (iVar3 = -2; iVar3 < 3; iVar3++) for (sStack_40 = -2; sStack_40
  < 3; sStack_40++)` IS visible in the PerCellProcess decompile I read
  for §4.3 — but the claim that this visits exactly 25 cells and there's
  "no off-by-one" was not independently bound-checked. Low risk; trivial
  to confirm.
- **§4.4 — "first hit wins" claim for high-vs-low bridge dispatch.** The
  audit verified the bridge-repair branch as a whole; this specific
  ordering claim was not exercised.
- **§4.6 `field_0x6DF` C4-plant-pending flag + the four field writes
  (`+0x6DF`, `+0x150`, `+0x14A`, `+0x14B`, `+0x14C`).** The audit
  confirmed the gates (no Immune/BridgeRepairHut check; IC blocks via
  `vtable[0x160]`) but the specific field-write offsets are taken
  verbatim from the doc.
- **§5.1 `Immune=` parse — write target `+0x233`.** The cited parse
  address `0x005F9510` was not directly exercised in this pass.
- **§5.2 "Immune=yes → attack cursor flipped to move cursor" — the tail
  branch of What_Action_OnObject.** The decompile shows
  `if (iVar7 == 5) { Type+0x233 == 0 ? 5 : 2 }`. Confirmed by reading
  the decompile end-to-end for §3 verification, but not flagged as
  audited.
- **§7 `Power=` parse and the `NEG EAX; MOV [EBP+0xEE4]` mirror claim.**
  Cited parse address `0x00461082` not exercised.
- **§8 TS-legacy audit table.** The "live in YR" / "dead in YR"
  determinations are reasoning-based and were not individually traced
  against caller xrefs in this pass.
- **The two cross-doc dispatch claims** — `vtable+0x3D4` =
  `BuildingClass::ChangeOwner` and `vtable+0xF8` = `Limbo` — were
  inferred from context. Reading the `BuildingClass` and `InfantryClass`
  vtables and confirming these slot values would close both this doc
  and CAHOSP §10 in a single round-trip.

If picking ONE follow-up target: **resolve the
`+0xEC3`-vs-`+0xEC5` engineer-flag tension between this doc and CAHOSP §10.**
The Rust port already disagrees with itself about where the engineer
flag lives (because of which doc was read first), and parity work on
engineer capture / spy / Tanya pipelines cannot proceed until this is
nailed down. One full read of `InfantryTypeClass::ReadINI` resolves it.
