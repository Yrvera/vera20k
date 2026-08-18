# Wall Overlay Destruction on Drive-Over (Movement-Side Crush) — Ghidra Research Report

- **Date:** 2026-07-19
- **Target:** What gamemd.exe does to a `Wall=yes` overlay when a crushing ground vehicle drives onto the cell.
- **Status:** VERIFIED-from-binary (core mechanism); one offset (LocomotorType +0x5B4) carried from a cross-referenced doc, flagged below.
- **Authority order:** binary → Ghidra → docs. Read-only; no Rust edits.

## TL;DR / Verdict

**The Rust port gap is REAL.** In gamemd, a wall overlay is destroyed as a **movement-side effect** the moment a qualifying vehicle finishes entering the cell — via `UnitClass__PerCellProcess @ 0x00739ec0` → `CellClass__DestroyOverlay(-1)` (forced, instant removal). This is a **separate code path from the weapon/warhead path** (`Apply_area_damage @ 0x00489280` → `CellClass__DestroyOverlay(damage)`). The Rust movement crush path (`src/sim/movement/bump_crush.rs`) only removes units/infantry; it never touches the wall overlay. So driving a Battle Fortress onto a wall currently leaves the wall intact — a real disparity.

## Key functions

| Function | Address | Role |
|---|---|---|
| `CellClass__DestroyOverlay` | `0x00480cb0` | The wall-overlay destruction routine (verified `decompile_function 0x00480cb0`) |
| `UnitClass__PerCellProcess` | `0x00739ec0` | Movement per-cell handler; drives the on-enter wall crush (verified `decompile_function 0x00739ec0`) |
| `UnitClass__Can_Enter_Cell` | `0x0073f0a0` | Passability query; wall-overlay branch (verified `decompile_function 0x0073f0a0`) |
| `TechnoTypeClass__ReadINI` | `0x00714ce3` | Writes `Crusher=` bool → TechnoType+0xD28 (verified `disassemble_function 0x00714c90`) |
| `OverlayTypeClass__ReadINI` | `0x005fe770` | Overlay field layout: `Wall=`→+0x2A8, `Crushable=`→+0x22D (verified `decompile_function 0x005fe770`) |

## 1. Can a ground vehicle enter a Wall overlay cell?

Yes, and it is gated on both a **movement zone** (zone-level pathing) and a **per-cell passability** check.

- **Zone-level (which units route a path *through* a wall):** governed by the unit's `MovementZone`. Only `MovementZone=CrusherAll` treats wall cells as part of the connected zone. Confirmed by the stock INI comment on the Battle Fortress: `ini/rulesmd.ini:6955` `MovementZone=CrusherAll;gs OmniCrush handles crushing tanks and such, this handles walls`. This matches the Rust reduced-zone matrix `src/sim/pathfinding/passability.rs` row 12 (CrusherAll) `Wall` column = 1 (passable) vs row 1 (Crusher) `Wall` = 2 (blocked).
- **Per-cell (`Can_Enter_Cell` wall branch, `0x0073f0a0`):** a `Wall=yes` overlay cell (`OverlayType+0x2A8 != 0`) is **not** returned as hard-impassable (code 7) when `(overlayCrushable(+0x22D) AND (Crusher(+0xD28) OR HasWeaponAbility)) OR (LocomotorType(+0x5B4) == 0xC/Drive)`; otherwise the unit needs a weapon that `CanFire` on the wall or the cell is impassable. When passable and the wall is allied, the branch assigns **code 4 (FriendlyWall)** — a *soft* block. Per `docs/research/pathfinding/UNIT_COLLISION_AND_REPATH_TRIGGERS_GHIDRA_REPORT.md` §3 (verified `0x004B2630`), a non-crusher **repaths around** code 4, while the `Crusher` flag (+0xD28) **downgrades code 4 → 0**, so the crusher drives onto the wall.

Net: BFRT (CrusherAll + Crusher) routes through and enters wall cells; a plain `Crusher=yes` drive tank does not zone-path across a wall but will crush a wall cell it does enter.

## 2. What happens to the wall — movement path vs weapon path

### Destruction routine: `CellClass__DestroyOverlay(damage)` @ `0x00480cb0`
(verified `decompile_function 0x00480cb0`)
- Returns 0 unless the cell's overlay is a `Wall` (`OverlayType+0x2A8`).
- **`damage == -1` (0xffffffff): forced, instant full removal** — bypasses the per-tick probabilistic gate.
- `damage >= 0`: probabilistic — `RandomRanged(0, Strength(+0x2A4)) > damage → return 0` (no removal this tick); otherwise bumps the damage level (`OverlayData += 0x10`) and only clears once the top damage level with lower-nibble 0 is reached.
- On clear: sets `field_0x50=-1`, `OverlayTypeIndex=-1`, `OverlayData=0`, `RecalcAttributes`, `AssignOrphanedCellZone`, updates the 4 cardinal neighbors' wall-connectivity frames (`CellClass__PostDestructionWallCleanup`), decrements the ore-neighbor count on all 8 neighbors. If this is the last damage level and `DamageLevels(+0x2A0) > 2`, chain-reacts into same-type wall neighbors (`DestroyOverlay(0xC8)`).

### Callers (verified `get_function_callers 0x00480cb0`)
- `Apply_area_damage @ 0x00489280` — **weapon/warhead path** (a `Wall=yes` warhead; passes real `damage`).
- `UnitClass__PerCellProcess @ 0x00739ec0` — **movement / drive-over path** (passes `-1`, forced). ← the mechanism this report is about.
- `BuildingClass__Limbo @ 0x00445880` — building removal/sell (clears `ProtectWithWall` walls).
- `FUN_0075f330 @ 0x0075f330` — a weapon-adjacent cell-impact path (reads `ChainReaction+0x2B1`, uses the weapon's `Damage` value); not movement.
- itself — chain reaction.

### The movement-side trigger (`UnitClass__PerCellProcess @ 0x00739ec0`, label `LAB_0073afd4`)
(verified `decompile_function 0x00739ec0`)

```c
cell = MapClass__Get_CellClass(current_cell);
if ( ( TechnoType->Crusher(+0xD28) != 0  ||  TechnoClass__HasWeaponAbility(0x11) )
     &&  cell->OverlayTypeIndex(+0x44) != -1
     &&  ( OverlayType->Crushable(+0x22D) != 0
           || ( OverlayType->Wall(+0x2A8) != 0  &&  TechnoType->LocomotorType(+0x5B4) == 0xC ) ) )
{
    GetCoords(&coords);
    VocClass__PlayAt(...);              // sound cue at the unit's coords (exact Voc unresolved)
    CellClass__DestroyOverlay(-1);      // forced, instant wall removal
    self->RockingForwardsPerFrame += 0.02;   // small forward tilt (cosmetic)
    self[1].field_0x195 = 0;
}
```

So the wall crush is a **movement-side effect**, driven by the same per-cell handler that resolves docking/entry, **not** the weapon damage path. It:
- deals **no damage to the vehicle**,
- removes the wall **instantly** (`-1` forced), not via the probabilistic strength gate the weapon path uses,
- plays a **sound cue**, and
- applies a **small forward rocking tilt** (`RockingForwardsPerFrame += 0.02`) — the "tips forward as it crushes the wall" cosmetic.

Timing: `PerCellProcess` runs on cell-entry completion (`FootClass__PerCellProcess(2)` = fully entered new cell), i.e. analogous to the Rust `DriveCrushPhase::FullyInCell`.

## 3. Which units / flags trigger it (stock YR)

Gate = `(TechnoType.Crusher(+0xD28) != 0 OR weapon-ability 0x11)` AND `Wall` overlay AND `LocomotorType == Drive(0xC)` (or an overlay that is `Crushable=yes`, which relaxes the locomotor/wall clause).

- **Battle Fortress `[BFRT]`** (`ini/rulesmd.ini:6917`): `Crusher=yes` (6935), `OmniCrusher=yes` (6936), `MovementZone=CrusherAll` (6955, comment "this handles walls"), `OmniCrushResistant=yes` (6966). It is a drive vehicle → satisfies the gate; it is also the only stock unit whose `MovementZone` lets it *route through* walls.
- Any other `Crusher=yes` drive vehicle also satisfies the destruction gate if it enters a wall cell, but with `MovementZone != CrusherAll` it will not zone-path across a wall to reach the far side.
- `TechnoType+0xD28 = Crusher=` is **verified**: `TechnoTypeClass__ReadINI` reads string `"Crusher"` (`@0x81bb58`) and stores the bool to `+0xD28` at `0x00714ce3` (`disassemble_function 0x00714c90`).
- `OverlayType+0x2A8 = Wall=`, `+0x22D = Crushable=` — **verified** field layout in `OverlayTypeClass__ReadINI @ 0x005fe770`. Stock wall overlays with `Wall=yes`: `[GAWALL]` (`ini/rulesmd.ini:12022/12031`), `[NAWALL]` (`12818/12827`), etc.
- **Not gated by OmniCrusher or OmniCrushResistant.** Those govern *unit/vehicle* crushing (`TechnoClass__CanCrushCheck @ 0x005f6cd0`, reads +0xD29/+0xD2A), not overlay walls.

## 4. Verdict (burden-of-proof honest)

**REAL GAP — DRIFT.** gamemd removes the wall overlay on drive-over via the movement path; the Rust port does not.

- gamemd: `UnitClass__PerCellProcess` → `CellClass__DestroyOverlay(-1)` on cell entry (verified above).
- Rust: `src/sim/movement/bump_crush.rs` (`cell_passable_after_crush`, `classify_drive_crush_phase`, `collect_crush_victims`) only evaluates *entity* occupants (units/infantry). `src/sim/pathfinding/passability.rs` row 12 lets CrusherAll enter the wall cell, but nothing in the movement path clears the overlay. Only the combat/warhead path (`src/sim/combat/mod.rs` `cell_has_wall_overlay` + `Wall=yes` warhead) removes walls in the port. Result: BFRT drives over a wall and the wall stays.

No INTERNAL-ONLY escape applies — this is a player-visible output (wall vanishes + sound + tilt in gamemd; wall remains in the port).

## 5. Implementation handoff (Rust movement crush path)

**Behavior to add** — when a unit **finishes entering** a cell (the `DriveCrushPhase::FullyInCell` moment in `bump_crush.rs` / `movement_tick.rs`):

1. **Gate:** the mover has `Crusher=yes` (existing `CrushCapability.regular_crusher`, from TechnoType+0xD28) **OR** the wall-destroy weapon ability, **AND** it is a ground/drive vehicle (LocomotorType Drive), **AND** the destination cell has a wall overlay (reuse `combat::cell_has_wall_overlay`). For a `Crushable=yes` overlay the drive-locomotor clause is not required.
2. **Effect:** remove the wall overlay from the resolved-terrain / overlay grid **immediately** (equivalent to `DestroyOverlay(-1)` forced removal — do **not** route through the probabilistic weapon damage). Recompute anything the port derives from wall overlays (zone/passability recalc for that cell + neighbors, wall-connectivity render frames, any ore-neighbor bookkeeping) to match `DestroyOverlay`'s cleanup.
3. **Sound:** emit the wall-crush Voc cue at the unit's cell (mirrors `VocClass__PlayAt`). Exact Voc index unresolved — pick the overlay/rules wall-crush sound; flag for a follow-up sound-parity pass.
4. **No unit damage.** The vehicle takes none.
5. **Cosmetic (optional, lower priority):** apply a small forward tilt (`RockingForwardsPerFrame += 0.02`) if the port models vehicle rocking.

**Acceptance test:** Place a `GAWALL` segment; order a Battle Fortress (`BFRT`) across it. Expect: BFRT paths through, the wall overlay is removed on the tick it fully enters the cell, a sound plays, BFRT HP unchanged. A plain (non-crusher) tank ordered at the same wall does **not** remove it and repaths around (code-4 soft block). Regression: a `Crusher=yes` tank that ends up on a wall cell removes it; a non-`Crusher` unit never does.

## Remaining uncertainty

- **LocomotorType `+0x5B4 == 0xC (Drive)`**: used identically in both the `Can_Enter_Cell` and `PerCellProcess` wall branches; the offset/value are carried from `docs/research/pathfinding/UNIT_CAN_ENTER_CELL_GHIDRA_REPORT.md` (field table) and not independently re-derived this session. Low risk for the verdict (BFRT and all stock wall-crushers are drive vehicles).
- **`HasWeaponAbility(0x11)`**: the alternate gate to `Crusher=`; ability index 0x11 not decoded here. Stock BFRT satisfies the `Crusher=` gate regardless, so this does not affect the primary case.
- **Exact Voc sound** played by the movement path is unresolved.
