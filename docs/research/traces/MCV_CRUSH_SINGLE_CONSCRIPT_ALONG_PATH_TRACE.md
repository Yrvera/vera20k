# MCV Crush Single Conscript Along Path — Trace Report

**Scenario:** Allied AMCV at cell (50, 50), facing East (0x40), right-click destination (55, 50).
Enemy Soviet Conscript (E1) at cell (53, 50), motionless, flat grass.
AMCV drives east; at (53, 50) it must crush the Conscript and continue to (55, 50).

**Sources used:**
- `docs/research/CRUSH_SYSTEM_GHIDRA_REPORT.md` — HIGH confidence, all offsets live-verified
- `docs/research/UNITCLASS_GHIDRA_REPORT.md`
- `docs/research/CELL_OCCUPATION_MARKING_GHIDRA_REPORT.md`
- `docs/research/LAYER_CLASS_GHIDRA_REPORT.md`
- `docs/research/ANIMATION_SOUNDS_GHIDRA_REPORT.md`
- Live Ghidra decompile: `UnitClass__PerCellProcess @ 0x741700`
- Live Ghidra decompile: `TechnoClass__CanCrushCheck @ 0x5F6CD0`
- Live Ghidra decompile: `DriveLocomotionClass__Process_Drive_Track @ 0x4B0F20`
- `ini/rulesmd.ini` — AMCV section at line 6969, E1 section at line 3713, Crush warhead at line 27104
- `src/sim/movement/bump_crush.rs`, `src/sim/movement/movement_tick.rs`,
  `src/sim/movement/movement_occupancy.rs`, `src/sim/pathfinding/cell_entry.rs`

---

## 1. INI Verification — Scenario Preconditions

### AMCV (`ini/rulesmd.ini` line 6969–7005)

| Key | Value | Notes |
|-----|-------|-------|
| `Crusher=yes` | present | `TechnoTypeClass+0xD28 = 1` in gamemd |
| `OmniCrusher=` | not set → false | `TechnoTypeClass+0xD29 = 0` |
| `MovementZone=Normal` | Normal | `TechnoTypeClass+0x5B4 = 0` |
| `Speed=4` | 4 | Standard drive speed |
| `ROT=5` | 5 | Rotation rate |
| `Weight=3.5` | 3.5 | Used in some pathfinding / passability logic |
| `Locomotor={4A582741}` | Drive locomotor | Standard wheeled drive |

**AMCV does NOT have `MovementZone=Crusher`** — it has `MovementZone=Normal` with a separate
`Crusher=yes` flag. This distinction matters critically for the Rust implementation.

### E1 — GI/Conscript (`ini/rulesmd.ini` line 3713–3763)

**Note:** In rulesmd.ini, `[E1]` is the GI (Allied infantry), not the Soviet Conscript.
The Soviet Conscript is `[E2]`. However for this scenario the assertion is that E1 has
`Crushable=yes` — verified present at line 3758 in the GI entry. The scenario uses
`E1 = Conscript` as a label shorthand; the actual E1 type in rulesmd.ini is crushable.

| Key | Value | Notes |
|-----|-------|-------|
| `Crushable=yes` | present (line 3758) | `ObjectTypeClass+0x22D = 1` |
| `CrushSound=InfantrySquish` | present (line 3725) | `ObjectTypeClass+0x1F0 = VocIdx(InfantrySquish)` |
| `DieSound=GIDie` | present (line 3743) | Separate die sound |
| `MovementZone=Infantry` | Infantry | Infantry zone — not a crusher zone |
| `Armor=none` | none | |
| `Strength=125` | 125 HP | Full HP at encounter |

### [CombatDamage] CrushWarhead (`ini/rulesmd.ini` line 825)

```
CrushWarhead=Crush
```

`[Crush]` warhead (line 27104–27111):
- `Verses=100%,100%,100%,...` — all armor classes take full damage
- `InfDeath=2` — infantry death animation index 2
- `ProneDamage=100%`
- `PenetratesBunker=yes`
- `AnimList=XGRYSML1,XGRYSML2,EXPLOSML,...` (explosion anims — not the infantry death anim)
- `CellSpread=.5`

---

## 2. Cell Entry Check at (53, 50) — What Predicate Decides Crush vs Block

### gamemd mechanism — verified from binary

**Entry point:** `DriveLocomotionClass::Process_Drive_Track @ 0x4B0F20`

When the AMCV's drive track steps arrive at the destination cell boundary, the locomotor
calls `Can_Enter_Cell` (`vtable+0x1AC`) on the owner (`UnitClass`). The result is a code 0–7:

- Code 0 (Clear) → proceed
- Code 3 (Crushable) → `MapClass::Check_Crushable_Obstacle` called; drive track continues
  (the actual crush death fires from `UnitClass::PerCellProcess`, not from this branch)
- Other codes → block/scatter/repath

**Crush entry gate in `Process_Drive_Track` (pre-crush scan at 0x4B18D6):**
The IsTrain-check path in Process_Drive_Track iterates the cell's FirstObject list and calls
`TechnoClass::CanCrushCheck` per occupant. If `CanCrushCheck` returns false (not crushable),
it applies the CrushWarhead (`g_RulesClass_Instance + 0xfac = RulesClass+0xFAC`) with 10000
damage to the victim and 0x14 damage to the crusher. This path (`*(char *)(iVar8 + 0xc94)` =
IsTrain field) is **not active for AMCV** (IsTrain=false). The AMCV follows the standard path.

**The actual crush-death trigger is `UnitClass::PerCellProcess @ 0x741700`.**
This is called by the movement system when the unit finishes entering a cell
(`entering == false` parameter). It is called AFTER the locomotor has committed the cell
transition, not before.

### The primary crush predicate — `TechnoClass::CanCrushCheck @ 0x5F6CD0`

Two independent checks (verified from decompile, live):

**Block 1 — OmniCrusher path:**
```c
if (crusher.TypeClass->OmniCrusher) {            // TechnoTypeClass+0xD29
    if (victim.IsOnMap) {
        if (!victim.TypeClass->OmniCrushResistant) {  // +0xD2A
            if (victim.WhatAmI() != 6) {              // not a Building
                if (!crusher.house.Is_Ally(victim)) {
                    if (!victim.IsBeingWarped()) {     // vtable+0x160
                        return true;
                    }
                }
            }
        }
    }
}
```

**Block 2 — Regular Crushable path (applies to AMCV + E1):**
```c
ObjectTypeClass* victimObjType = victim.GetObjectType();    // vtable+0x88
if (victimObjType->Crushable) {                              // +0x22D
    if (victim.IsOnMap) {
        if (victim[0x2A4] == 0) {   // NOT deployed/prone state byte
            if (!crusher.house.Is_Ally(victim)) {
                if (!victim.IsBeingWarped()) {
                    return true;
                }
            }
        }
    }
}
return false;
```

For this scenario:
- `E1.Crushable = true` ✓
- `E1.IsOnMap = true` ✓
- `E1[0x2A4] = 0` (motionless, not deployed) ✓ — Conscript is not GGI, no deploy state
- `crusher.house != E1.house` (Allied vs Soviet) ✓
- `E1.IsBeingWarped() = false` ✓

→ `CanCrushCheck` returns **true**. Conscript is crushable.

**Note on AMCV's Crusher=yes flag:** `PerCellProcess` gates the entire crush loop on:
```c
if (AMCV.TypeClass->Crusher /* +0xD28 */ || HasWeaponAbility(0x11 /* CRUSHER */))
```
AMCV has `Crusher=yes` → `TypeClass+0xD28 = 1` → this gate passes. The crush loop runs.

### Distance check

After `CanCrushCheck` passes, the inner loop checks:
```c
objCoords = victim.GetCoords();       // vtable+0x48
distSq = DistanceSquared(crusher, objCoords);  // @ 0x5F6560
if (distSq > 0x3FFF) skip;           // > 16383 → skip (threshold: ~128 leptons)
```

When the AMCV has fully entered cell (53, 50), both AMCV and E1 are centered at the same
cell, so distance ≈ 0 << 16383. The distance check passes.

### InLimbo check

```c
if (victim[0x8D] != 0) skip;   // in limbo → skip
```
E1 is on the map, not in limbo → passes.

**Summary of crush predicate inputs (numerical):**

| Input | Value | Source |
|-------|-------|--------|
| `AMCV.TypeClass->Crusher` | 1 (true) | `[AMCV] Crusher=yes` → `TechnoTypeClass+0xD28` |
| `E1.TypeClass->Crushable` | 1 (true) | `[E1] Crushable=yes` → `ObjectTypeClass+0x22D` |
| `E1.TypeClass->OmniCrushResistant` | 0 (false) | not set |
| `AMCV.TypeClass->OmniCrusher` | 0 (false) | not set |
| Distance at crush tick | ~0 leptons | both centered at (53,50) |
| Distance threshold | 0x3FFF = 16383 sq leptons | `FUN_005f6560` |
| E1 deployed state byte `+0x2A4` | 0 | not deployed, motionless |
| E1 IsBeingWarped | false | no chrono in progress |
| House ally check | Allied ≠ Soviet | enemies → NOT allied |

No weight comparison is used in the crush decision. `Weight=` is not read in `CanCrushCheck`
or `PerCellProcess`. There is no weight threshold for regular infantry crush.

---

## 3. Death Sequence in PerCellProcess (entering == false)

Verified from live decompile at `0x741700`. The full crush-kill sequence for one victim:

```c
piVar6 = victim->NextObject;          // save next occupant
bVar1 = true;                         // didCrush = true
victim->WhatAmI();                    // RTTI check (0xF = InfantryClass) — return val ignored here
uVar7 = 0;                            // Ghidra artifact: EAX from GetObjectType holds CrushSound idx
local_18 = crusher->Coords.X;        // crusher X coordinate (lepton)
iStack_14 = crusher->Coords.Y;       // crusher Y coordinate (lepton)
iStack_10 = crusher->Coords.Z;       // crusher Z coordinate (lepton)
victim->GetObjectType(0);             // returns ObjectTypeClass*, EAX = CrushSound voc index
VocClass__PlayAt(/* EAX = CrushSound idx */, &crushCoords, 0);  // play at crusher position
victim->FreeAllMindControlCaptures(); // vtable+0x170
victim->RecordKill(crusher);          // vtable+0xE0 — score/EVA
victim->MarkForDeletion(0);           // vtable+0x124
victim->Destroy();                    // vtable+0xD4
victim->RemoveFromGame();             // vtable+0xF8
```

**Note on VocClass call:** Ghidra shows `VocClass__PlayAt(uVar7)` where `uVar7 = 0`. This
is a Ghidra register-value propagation artifact. The stack layout shows `local_18/iStack_14/
iStack_10` holding the crusher's XYZ coordinates (crusher+0x27/0x28/0x29 × 4). The calling
convention uses EAX from the preceding `vtable+0x88` (GetObjectType) call as the first
parameter to `VocClass__PlayAt` — carrying the CrushSound voc index from the victim's
ObjectTypeClass+0x1F0. The coordinates passed are the **crusher's position**, not the victim's.

**Death event timing:** Victim death is committed at the first crush tick (the tick when
`entering == false` fires, i.e., the tick the AMCV finishes fully entering cell (53,50)).
The Conscript is removed from the game (`RemoveFromGame`) within the same PerCellProcess
call. It does not persist to the next tick.

**Damage:** There is NO `ReceiveDamage` call in the standard PerCellProcess crush path.
The victim is killed directly via `MarkForDeletion + Destroy + RemoveFromGame`, bypassing
the normal HP damage pipeline. Damage = instant 100% HP loss (direct removal). No CrushWarhead
is applied in this path.

**Exception:** The `CrushWarhead` at `RulesClass+0xFAC` is applied ONLY in
`DriveLocomotionClass::Process_Drive_Track` when `IsTrain=yes` and `CanCrushCheck` fails —
this is a different code path (0x4B18D6) that deals with non-crushable obstacles being run
over by trains. It does NOT apply to the standard AMCV-vs-E1 crush.

**Damage to AMCV:** Zero. `PerCellProcess` applies no damage to the crusher. The
`Process_Drive_Track` train path applies 0x14 (20) damage to the crusher, but AMCV is not
a train and that path is not taken.

---

## 4. Sound Cue at Crush

### Source: `[E1] CrushSound=InfantrySquish`

`CrushSound` is an `ObjectTypeClass` key (INI parser `ObjectTypeClass::ReadINI @ 0x5f9400`,
binary lookup via `VocClass::FindByName`, stored at `ObjectTypeClass+0x1F0` as a VocClass
index). The sound played comes from the **victim's** `CrushSound=` field, not the crusher's.

The AMCV also has a `CrushSound=` entry? Let me clarify: the AMCV section in rulesmd.ini
(line 6969–7005) does NOT have a `CrushSound=` key. Looking at the code, `PlayAt` is called
with the value from `victim->GetObjectType()` followed by `+0x1F0`, which is the **victim's**
CrushSound. E1's `CrushSound=InfantrySquish`.

**Sound event summary at crush tick:**

| Sound | INI Key | Source | Timing |
|-------|---------|--------|--------|
| `InfantrySquish` | `[E1] CrushSound=InfantrySquish` | victim ObjectTypeClass+0x1F0 | Crush tick (PerCellProcess, entering==false) |
| `GIDie` | `[E1] DieSound=GIDie` | RecordKill → death handling | Same tick (ReceiveDamage/RecordKill chain) |

The `VocClass::PlayAt` in `PerCellProcess` plays the CrushSound at the **crusher's
coordinates** (crusher+0x9C/0xA0/0xA4), not the victim's position. This means the sound
origin is the AMCV's cell center, not where the Conscript was standing.

**No separate [General] CrushSound exists** for the global override case. The per-unit
`CrushSound=` on the victim is the authoritative source. The AMCV's own CrushSound (if any)
is not used in this path.

---

## 5. Squish Animation

### InfDeath=2 → What animation plays?

The `[Crush]` warhead has `InfDeath=2`. In gamemd, when an infantry is killed by a warhead
with `InfDeath=N`, `InfantryClass::Killed` (virtual) selects the Nth entry in a global
infantry death animation table (`g_InfantryDeathAnims`). Index 2 corresponds to the
"flatten/squish" animation type.

**However:** In the standard PerCellProcess crush path, the death is applied via direct
`MarkForDeletion + Destroy + RemoveFromGame` — NOT through `ReceiveDamage`. The CrushWarhead
(`[Crush]` with `InfDeath=2`) is NOT applied in this path. The `InfantryClass::Killed`
virtual function that spawns the InfDeath animation IS called as part of the `Destroy`
chain (`vtable+0xD4`) for infantry, which internally invokes the death animation sequence.

The specific animation name for InfDeath=2 in the artmd.ini death anim table is not directly
visible as a named entry; it is selected at runtime from the `g_InfantryDeathAnims` array
indexed at slot 2. Based on the standard YR asset set, InfDeath=2 corresponds to
the blood-splat / flatten squish animation (visually: infantry becomes a flat red mark).

**No art.ini CrushSound or StartSound on the death anim** is separately verified here —
if the death anim has a `StartSound=`, it would play in addition to the CrushSound from
PerCellProcess. This is unverified in this trace.

### Z-order of squish animation

From `AnimClass::GetLayer @ 0x00424CB0` (verified, HIGH confidence):
```c
int AnimClass__GetLayer(this) {
    if (this->field_0xCC != 0) return 2;   // attached to owner → Ground layer
    if (this->AnimType != NULL) return this->AnimType->Layer;  // AnimType+0x364
    return 3;  // default → Ground
}
```

Infantry death anims are NOT attached to an owner (the infantry is being destroyed).
They use the Layer field from their AnimType, defaulting to Ground (layer 2) when no
explicit `Layer=` is set. The layer enum: 0=Underground, 1=Surface, 2=Ground, 3=Air(?),
4=Top. Ground layer (2) renders BELOW vehicles (Surface layer 1).

**Result:** The squish animation draws at ground level, UNDER the AMCV sprite.
The AMCV continues driving over the top of the animation.

---

## 6. AMCV Facing During and After Crush

**No facing change.** The `PerCellProcess` crush loop contains no facing modification for
the crusher. The only facing-related code in the crush path is for the infantry-pickup
special case (`*(char *)(piVar4[0x1b0] + 0xec6)` flag), which is a different branch —
not applicable here since E1 is an enemy, not a transport-absorbable friendly infantry.

The AMCV entered cell (53,50) facing East (0x40). After PerCellProcess completes, facing
remains 0x40. The drive track continues toward (55,50) without any heading adjustment.

---

## 7. AMCV Speed During Crush

**No speed change for infantry crush.** The `TiltsWhenCrushes` cosmetic effect (`bVar1 =
true` sets a tilt of `0xBD4CCCCD` ≈ -0.05 radians at crusher+0x334) is checked via
`vtable+0x45C` after the crush loop completes, but `TiltsWhenCrushes` is not set in the
`[AMCV]` section. Even if set, it is a visual tilt only — not a speed reduction.

The `IsOnWall` / `CrushingWallDeceleration` flag (`FootClass+0x6B5`) causes deceleration
when crushing **walls** (overlay walls, fences). This is set via `Process_Drive_Track` when
`MovementZone=CrusherAll` and a wall overlay is present. AMCV has `MovementZone=Normal`
and a Conscript is not a wall overlay. This flag is NOT set.

**Result:** AMCV maintains its normal drive-track speed through cell (53,50). No speed
reduction for infantry crush.

---

## 8. Tick Count for Crush vs Normal Cell Entry

**Timing:** The `PerCellProcess` crush kill fires in the same tick as cell entry completion.
It is not deferred. The sequence within one tick:

1. `DriveLocomotionClass::Process_Drive_Track` advances drive track steps, commits AMCV
   to cell (53,50)
2. `UnitClass::PerCellProcess` is called with `entering==true` first (scatter phase)
3. `UnitClass::PerCellProcess` is called with `entering==false` (crush phase) — E1 is killed
4. `RemoveFromGame` removes E1 from the cell list within the same tick

Compared to entering an empty cell: exactly the same number of ticks. The crush does NOT
add a delay tick. Cell transition time = identical to normal movement.

The scatter phase (`entering==true`) fires `CellClass::Scatter_Objects` on occupants.
For a Conscript with `Crushable=yes`, the scatter call would tell E1 to move away.
However, the scatter is immediately followed (same tick, next `PerCellProcess` call) by
the crush kill. Whether the scatter has any observable effect before the crush fires
depends on locomotor scheduling — in practice, for infantry in a cell the crusher
is already entering, the crush kill supersedes the scatter.

---

## 9. Parity Comparison: gamemd vs Rust

### DRIFT — Critical: AMCV does not crush in Rust

**Severity: CRITICAL — fires every time any vehicle with `Crusher=yes` and `MovementZone≠Crusher`
attempts to crush infantry.**

In gamemd, `UnitClass::PerCellProcess @ 0x741700` gates the crush loop on:
```c
TechnoTypeClass+0xD28 (Crusher=yes) OR HasWeaponAbility(0x11)
```
AMCV has `Crusher=yes` → the loop runs → Conscript is crushed.

In Rust (`src/sim/movement/bump_crush.rs`, `can_crush` function):
```rust
pub fn can_crush(mover_zone: MovementZone, ...) -> bool {
    match mover_zone {
        MovementZone::CrusherAll => true,
        MovementZone::Crusher | ... => {
            target_category == EntityCategory::Infantry && target_crushable && ...
        }
        _ => false,   // ← AMCV hits this branch (MovementZone::Normal)
    }
}
```

The Rust codebase does NOT parse `Crusher=yes` from rules.ini. There is no `crusher: bool`
field on `ObjectType` or `GameEntity`. `omni_crusher` is parsed from `OmniCrusher=` (a
different key). AMCV with `MovementZone=Normal` + `Crusher=yes` falls into `_ => false` and
cannot crush anything.

**Units affected in stock YR:**
- AMCV (`MovementZone=Normal`, `Crusher=yes`)
- SMCV (Soviet MCV — needs verification but likely same pattern)
- Any other unit with `Crusher=yes` and non-Crusher MovementZone

**Fix:** Parse `Crusher=yes` into a dedicated `crusher: bool` field on `ObjectType` / `GameEntity`.
In `can_crush`, add a `mover_crusher: bool` parameter. When `mover_crusher == true`, treat the
mover as if its zone were at minimum `MovementZone::Crusher` for infantry crush checks.
The existing MovementZone-based check covers units like CMIN (which has `MovementZone=Crusher`
AND `Crusher=yes`), but not AMCV (which has only `Crusher=yes`).

### DRIFT — Sound placement: crusher coords vs victim coords

**Severity: LOW — audible as subtle spatial positioning error.**

gamemd plays `CrushSound` at the **crusher's** coordinates (`crusher+0x9C/0xA0/0xA4`).
Rust `emit_crush_kill_sounds` uses `victim.position.rx/ry`:
```rust
let rx = victim.position.rx;
let ry = victim.position.ry;
```

When the crush fires (entering==false), the AMCV has just completed entering cell (53,50),
so its cell position equals the victim's cell. The sub-cell positions may differ slightly
(AMCV at cell center, Conscript at sub-cell 2/3/4), but at cell resolution the coordinates
are identical. The discrepancy is sub-cell (< 256 leptons). This is a very small spatial
error unlikely to be audible, but it is technically a drift from the binary.

### MATCH — Ally check

Rust `can_crush` is called from `classify_occupied_cell_with_layers` which uses the
`alliances` map. AMCV (Allied) vs E1 (Soviet) → not allies → can crush. This matches
gamemd's `HouseClass::Is_Ally_ByObject` check.

### MATCH — No damage to AMCV

Rust applies `victim.health.current = 0; entities.remove(victim_id)` with no damage to
the mover. Matches gamemd PerCellProcess (no damage to crusher in the standard path).

### MATCH — Instant kill this tick

Rust defers to post-loop `crush_kills` processing in `tick_movement_with_grids`, applying
kill in the same sim tick as the cell entry. Matches gamemd's same-tick RemoveFromGame.

### MATCH — No facing change

Rust movement loop does not alter `entity.facing` during crush. Matches gamemd.

### MATCH — No speed change for infantry crush

Rust has no speed reduction path for infantry crush (only for wall crush via CrushingWallDeceleration).
Matches gamemd.

### DRIFT — Scatter timing vs crush timing (entering==true phase)

**Severity: LOW — scatter fires before crush; if scatter succeeds it displaces the victim
before crush fires.**

In gamemd, `PerCellProcess` with `entering==true` calls `CellClass::Scatter_Objects` on
the occupant list. For a Crushable infantry that is motionless, the scatter tells it to
move away. Then `PerCellProcess` with `entering==false` fires the crush. In practice for
a single-tick scenario, the scatter command is issued in the same tick the crush fires, so
the Conscript receives a movement order but is killed before it can act on it.

In Rust, `detect_deferred_cell_check` fires on the vehicle's next-cell arrival. The
`handle_deferred_occupancy` function calls `classify_occupied_cell_with_layers`, which
checks crush first (`collect_crush_victims`), then scatter only if crush returns empty.
Because the Rust `can_crush` currently returns false for AMCV (see Critical DRIFT above),
the Rust code falls through to `FriendlyStationary` or similar classification. Even after
fixing the `Crusher=yes` parsing, the Rust code does NOT implement the `entering==true`
scatter phase before the `entering==false` crush phase. This is a missing behavior: in
gamemd, the pre-crush scatter phase first tells the infantry to flee, which can have
observable effects if multiple infantry are in the cell.

### UNCHECKED — InfDeath death animation (animclass not implemented)

AnimClass is not implemented in the Rust engine (confirmed: `ANIMCLASS_SPAWN_PATHS_GHIDRA_REPORT.md`
§11). The squish animation does not spawn. This is a known gap, not specific to crush.

### UNCHECKED — DieSound/VoiceDie from RecordKill chain

Whether the GIDie DieSound and VoiceDie fire correctly through the kill path needs separate
verification once the full death audio pipeline is wired. Not addressed in this trace.

---

## 10. Summary Table

| Observable Output | gamemd behavior | Rust behavior | Status |
|-------------------|-----------------|---------------|--------|
| AMCV recognizes E1 as crushable | Yes — `Crusher=yes` flag at `+0xD28` gates loop | No — `MovementZone=Normal` → `can_crush` returns false | **DRIFT (Critical)** |
| Crush predicate: Crusher flag | `TechnoTypeClass+0xD28` (`Crusher=yes`) | Not parsed — missing field | **DRIFT (Critical)** |
| Crush predicate: Crushable flag | `ObjectTypeClass+0x22D` | `e.crushable` field ✓ | MATCH |
| Crush predicate: no weight comparison | Confirmed — no weight check in CanCrushCheck | Correct — no weight check | MATCH |
| Crush predicate: deployed check | `victim[+0x2A4] == 0` (not deployed) | `is_low_silhouette_for_crush` ✓ | MATCH |
| Conscript death: same tick | RemoveFromGame in PerCellProcess, entering==false tick | `crush_kills` post-loop same tick | MATCH |
| Conscript death: instant, no HP damage | MarkForDeletion+Destroy+RemoveFromGame directly | `health=0; entities.remove()` | MATCH |
| Conscript death: emitted at first crush tick | Yes — PerCellProcess entering==false | Yes — same tick | MATCH |
| Damage to AMCV | Zero | Zero | MATCH |
| AMCV facing after crush | 0x40 East (unchanged) | Unchanged | MATCH |
| AMCV speed during crush | Unchanged | Unchanged | MATCH |
| CrushSound: from victim ObjectType | `[E1] CrushSound=InfantrySquish` | `obj.crush_sound` from victim ✓ | MATCH |
| CrushSound: played at crusher coords | Crusher position (lepton precise) | Victim's cell position (cell-grid) | DRIFT (Low) |
| Squish anim: ground level, under AMCV | AnimType Layer=Ground (2), below Surface units | AnimClass not implemented | UNCHECKED |
| Ticks for crush vs empty cell | Same — no extra tick | Same | MATCH |
| Pre-crush scatter phase (entering==true) | Scatter issued to E1 before kill | Not implemented as two-phase | DRIFT (Low) |

---

## 11. Required Fix

**Primary fix:** Parse `Crusher=yes` from `rules.ini` into `ObjectType::crusher: bool` and
propagate to `GameEntity::crusher: bool` via world_spawn. Update `can_crush` to accept a
`mover_crusher: bool` parameter. When true, treat the mover as a standard crusher (equivalent
to `MovementZone::Crusher` for infantry-crush purposes). Pass this flag through
`collect_crush_victims → can_crush` call chain.

Concretely in `bump_crush::can_crush`:
```rust
pub fn can_crush(
    mover_zone: MovementZone,
    mover_omni_crusher: bool,
    mover_crusher: bool,   // ← new: from Crusher=yes INI flag
    target_category: EntityCategory,
    target_crushable: bool,
    target_low_silhouette: bool,
    target_omni_crush_resistant: bool,
) -> bool {
    // ... existing checks ...
    match mover_zone {
        MovementZone::CrusherAll => true,
        MovementZone::Crusher | MovementZone::AmphibiousCrusher | ... => {
            target_category == EntityCategory::Infantry && target_crushable && !target_low_silhouette
        }
        _ if mover_crusher => {
            // Crusher=yes with non-Crusher zone: same infantry-only crush as Crusher zone
            target_category == EntityCategory::Infantry && target_crushable && !target_low_silhouette
        }
        _ => false,
    }
}
```

All call sites for `collect_crush_victims`, `cell_passable_after_crush`, and the pathfinding
`mover_is_crusher` check must be updated to pass the new flag.

The `Crusher=yes` flag also gates the entire crush loop in `PerCellProcess`. The Rust
pathfinding `mover_is_crusher` (lines 328–335 of movement_occupancy.rs) currently uses
`MovementZone` only — this must also incorporate the new `crusher` bool.

---

*Investigated by subagent slot 4 of /trace-swarm batch. Binary addresses verified live
in Ghidra MCP. INI values from `ini/rulesmd.ini` in-repo. Rust code from
`src/sim/movement/bump_crush.rs`, `movement_tick.rs`, `movement_occupancy.rs`,
`pathfinding/cell_entry.rs`.*
