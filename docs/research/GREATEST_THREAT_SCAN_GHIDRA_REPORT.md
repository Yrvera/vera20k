# FootClass::Greatest_Threat_Scan — Ghidra Research Report

**Address:** `0x004D5690` (FootClass::Greatest_Threat_Scan, 737 lines, vtable slot +0x53C)
**Companion:** `0x004D9920` (FootClass::Greatest_Threat, 14-line wrapper, vtable slot +0x3C4)
**Research date:** 2026-04-23
**Source:** Ghidra MCP live decompilation of gamemd.exe
**Confidence:** HIGH overall (full decompilation of both functions and all non-trivial callees; magic constants dumped from memory; spiral-fan offset table fully decoded)
**Active in YR:** Yes (vtable slot is live on all FootClass subclasses — InfantryClass, UnitClass, AircraftClass)

---

## 1. Overview

**`FootClass::Greatest_Threat_Scan` is a misnomer — it is not a target scanner.** It is the **approach driver / firing-position search** for mobile units that *already* have a committed TarCom. It takes the current target and decides one of:

1. Stand still (already in weapon range with LOS) — fall through and let the per-frame firing pipeline handle the shot.
2. Step to a nearby cell that puts the unit into weapon range (angular fan search around the bearing toward the target, plus an 8-compass fallback ring). Sets NavCom via `Set_Destination`.
3. For `CloseRange=yes` infantry (Tanya, Yuri, SEAL, demo carriers): snap into the target's cell or an adjacent infantry subcell via `PlaceInfantryInCell`, bypassing the pathfinder.
4. Abandon TarCom and transition via `OnArrival(0, 1)` (to Guard/idle) when the target is unreachable, invalid, or the unit cannot fire.
5. Drop the archive approach target when the live TarCom has drifted too far from where pursuit began (gated by `CanRecalcApproachTarget` on the target's TypeClass).

> **Errata (2026-04-23 follow-up):** earlier passes of this doc and `TARGET_ACQUISITION_GHIDRA_REPORT.md` labeled `TypeClass+0xD33` as `CanBeScattered` and `TypeClass+0xD34` as `OpportunityFire`. Both are wrong. The correct names — verified by disassembling `TechnoTypeClass::ReadINI` and reading the exact strings pushed to `CCINIClass::ReadBool` — are:
> - `TypeClass+0xD33` = `CanApproachTarget` (bool, default yes; `no` on `[SPY]` in rulesmd.ini:6836)
> - `TypeClass+0xD34` = `CanRecalcApproachTarget` (bool)
> - The real `OpportunityFire` lives at `TypeClass+0x6AF` and is not read by this function at all.
>
> See `OPPORTUNITY_FIRE_GHIDRA_REPORT.md` for the full trace. The **behavior** described in §3(A)ii and §3(D) is unchanged — only the INI key names that gate those branches were wrong. Call-site sections and the offset table in §7 have been updated in place.

The name comes from the vtable's parallel slot `+0x3C4 Greatest_Threat` (the actual threat scanner). Whoever named the FootClass slot `+0x53C` reused "Greatest_Threat" because it is *invoked* in the target-acquisition path, even though it does no threat evaluation itself. Several existing repo docs (`FOOTCLASS_MISSION_HANDLERS`, `FOOTCLASS_VTABLE_COMPLETE`, `TARGET_ACQUISITION`) carry this misnomer forward; the Mission_Attack report §3 is correct.

The companion wrapper at `0x004D9920` (`FootClass::Greatest_Threat`, vtable +0x3C4) is the FootClass override of `TechnoClass::Greatest_Threat`. It just forces threat-flag bit 0 (weapon-range scan) while a scanning flag is set, delegates to the base class, and clears the flag on no-target. **The actual threat scoring/scanning lives in `TechnoClass::Greatest_Threat @ 0x006F8DF0`** (documented in `ra2-rust-game-docs/TARGET_ACQUISITION_GHIDRA_REPORT.md`).

---

## 2. Signatures

```c
// vtable +0x53C — the approach driver.
int __thiscall FootClass::Greatest_Threat_Scan(
    FootClass* this,
    char dry_run      // param_2: if non-zero, do not mutate NavCom / TarCom; return what it WOULD do
);
// Returns: zero on no-op / abandon, otherwise a target pointer (current TarCom) or a CellClass* that
// the caller can feed back into Set_Destination. Return value is consulted by callers to decide whether
// the approach succeeded.

// vtable +0x3C4 — the threat-scan override (thin wrapper).
void __thiscall FootClass::Greatest_Threat(
    FootClass* this,
    uint threat_flags,
    int* scan_origin,
    char enemy_only
);
```

### `param_2` (dry_run)

Every mutation inside `Greatest_Threat_Scan` is gated on `param_2 == 0`. When `param_2 != 0`, the function walks through the same decision tree but never touches TarCom, NavCom, destination coords, Set_Coord_Direct, or the scatter/subcell state. Callers use this as a "would I do anything useful?" probe. In the current binary **both direct callers pass `param_2 = 0`** (see §8), so the dry-run path is effectively dead in normal play — but it is reachable via the vtable from any caller that wants a preview (Mission_Attack uses `param_2 = 0` too). Treat as "implemented for future use but currently always called with mutation enabled."

---

## 3. Control flow — high level

Below is a summary of the dispatch. Exact addresses in parentheses.

```
ENTRY (0x004D5690)
 │
 │── (A) Gate checks (0x004D56AE–0x004D57EA)
 │    · NavCom == TarCom → 0  (already walking onto the target)
 │    · TarCom == NULL  → 0
 │    · Get TarCom's Likely_Coord (vtable +0x2E4)       → scan_origin
 │    · Read weapon range via vtable +0x168             → iStack_130
 │    · If range > 511 leptons: subtract 128            (bias: firing pos 0.5 cells inside range)
 │    · Can_Fire_At (vtable +0x3A8) = OK?
 │       · OK  → scan proceeds
 │       · NOT OK → test fall-through cases:
 │             (i)  mission == 6 (STOP)                    → abandon branch
 │             (ii) CanApproachTarget flag (TypeClass+0xD33)
 │                  AND NOT actively-attacking (FUN_0070FEB0 reads this+0x1CC)
 │                  AND NOT +0xB9 (lock flag)
 │                  AND NOT +0x82 byte flag (lock flag)
 │                     → continue to approach logic with cVar3=1 (scatter mode)
 │             (iii) mission == 1 (GUARD)                  → approach logic
 │             (iv)  mission == 11 (ENTER)
 │                   AND owner is player-controlled       → approach logic
 │             (v)   mission == 15 (HARVEST)
 │                   AND NOT player-controlled            → abandon branch
 │             else → abandon branch
 │
 │── ABANDON (LAB_004D571F)
 │    if dry_run: return 0
 │    vtable +0x3C8 (Set_ArchiveTarget, nullptr, 0)     // clear archive target
 │    vtable +0x480 (OnArrival, 1)                      // transition out (typically to Guard)
 │    return 0
 │
 │── (B) Building-blocker early-snap (0x004D57EA–0x004D586A)
 │    · Only for infantry-type units whose TypeClass+0xE12 byte is set
 │      (flag-set for melee/suicide/civilian-infantry classes)
 │    · If attacker's current cell has NO building (FUN_00487C10 passes)
 │      AND 3D distance to target + 512 leptons (2 cells) <= weapon_range,
 │      shrink the approach radius to (distance + 512).
 │    (This lets melee/demo infantry drop a tighter search ring when they
 │    are already close; avoids a full spiral outward.)
 │
 │── (C) Locomotor piggyback probe (0x004D586A–0x004D58D2)
 │    · QueryInterface(IPiggyback) on the unit's locomotor.
 │    · Compare the piggybacked locomotor class-GUID against
 │      CLSID_WalkLocomotion @ DAT_007E9AC0.
 │    · If match: cVar22 = 1 (is walking locomotion — tells later code
 │      to use per-infantry subcell snapping).
 │    · Acquires an additional locomotor reference (released on exit).
 │
 │── (D) Archive-target retarget abort (0x004D58DA–0x004D5A1A)
 │    Entered only when this+0x5A4 (field 0x169) is non-zero (an archive
 │    destination / secondary target cell is stashed).
 │    · If TarCom->TypeClass+0xD34 (CanRecalcApproachTarget) AND wrapper-flag byte
 │      clear, compute 3D distance between TarCom's coord and the stashed
 │      +0x5A4 coord.
 │    · If distance > RulesClass+0xDF8 * approach_radius_cells
 │      AND dry_run==0 → clear this+0x5A0 and this+0x5A4 (drop the stashed
 │      archive approach target).
 │    · Continue to approach logic unless the locomotor reports it is busy
 │      (vtable +0x54). If busy and the opp-fire target was NOT cleared,
 │      early-return 0.
 │
 │── (E) Approach logic (0x004D5A1A–0x004D689A)   ← the main body
 │    see §4 below
 │
 │── (F) NavQueue consumption (end of the approach body)
 │    · If this+0x598 (NavQueue count) > 0: dequeue first entry,
 │      call OnArrival(cell, 0) to step toward it, shift the queue,
 │      release the extra locomotor reference, return the dequeued cell.
 │
 │── (G) Final fallback — Find_Nearby_Passable_Cell (0x004D68FC end)
 │    Reached only if approach logic completed without finding any cell.
 │    · HunterSeeker units (TypeClass+0xD27) and engineer-type infantry
 │      (mission == 15 AND HouseClass+0xEC6) bypass this and march straight
 │      at TarCom: Set_Destination(TarCom, 1), return TarCom.
 │    · Otherwise: call FootClass::Find_Nearby_Passable_Cell (0x0056DC20)
 │      searching up to 32 cells outward around TarCom's cell, pick the
 │      cell that passes CellRect::CheckPassability for our SpeedType/zone.
 │    · Emit Set_Destination(picked_cell, 1) or OnArrival(0, 1) if no cell.
```

The above is the skeleton; §4 covers (E) in detail because that is where all the spatial search happens.

---

## 4. The approach scan (section E in detail)

### 4.1 Setup phase

```c
// Approach radius baseline (uStack_140)
if (TarCom is a BuildingClass (type 6)):
    radius_leptons += (foundation_w + foundation_h) * 0x40       // 64 per cell
radius_leptons = Math::ftol(radius_leptons)                       // already in leptons
radius_leptons = sign_clamp(radius_leptons)                       // negatives → 0
if (0 < radius_leptons < 256):    radius_leptons = 256            // floor to 1 cell

// scan_origin = TarCom's CoordStruct (vtable +0x48)
// self_coord  = this->CoordStruct (vtable +0x48)
// bearing_byte (uStack_b0) = ((atan2(dy, dx) >> 7) + 1) >> 1   — 8-bit compass bearing

// Bridge bias (piStack_fc low byte = "target_on_bridge" for InRange check)
if (TarCom type == 11 /* AircraftClass */):
    piStack_fc low byte = (AircraftCoord.Z >> 8)
else if (TarCom.field_0x14 bit 1 set):
    piStack_fc low byte = TarCom+0x8C byte                         // already set on-bridge flag

// CloseRange weapon-range clamp:
//   if unit-type is infantry (type 0xF) and TypeClass+0x695 (CloseRange=yes)
//   and weapon_range < 332.8 leptons (1.3 cells):
//       weapon_range_clamped = 0x14C (332 leptons)              // DAT_007E9248
```

**Magic #1 — the -128 weapon-range bias:** when the raw weapon range is greater than 512 leptons (2 cells), the function subtracts 128 leptons (0.5 cells) from the effective range used for the approach. This gives the unit half-a-cell of breathing room so it parks slightly inside the kill range rather than right at the edge, which reduces jitter when the target moves. For range ≤ 2 cells (short-range weapons) no bias is applied — those units need every lepton they can get.

### 4.2 CloseRange subcell snap (pre-spiral)

Before the angular fan search, infantry with `CloseRange=yes` check whether they are already within `DAT_007E9240 = 384 leptons` (1.5 cells) of the target's cell center:

```c
if (TypeClass+0x695 CloseRange) {
    // distance from this's cell-center to TarCom's cell-center (2D, in leptons)
    d_to_targetcell = sqrt(dx*dx + dy*dy)
    if (d_to_targetcell <= 384.0) {
        // Try to place into TarCom's cell as an infantry subcell
        candidate_coord = CellClass::PlaceInfantryInCell(TarCom_cell, subcell=1)
        // If that subcell is closer to the target than the current position:
        if (dist(candidate_coord, target) < d_to_targetcell) {
            if (!dry_run)
                this->Locomotor->Set_Coord_Direct(candidate_coord)  // vtable +0x78
            mark approach_done = true
        }
    }
}
```

This snap is the authentic behavior for Tanya / SEAL / Yuri / demolition infantry: when they arrive at the target cell, they teleport into a subcell instead of walking further, avoiding visible repositioning jitter at the moment of attack.

### 4.3 Angular fan search (the spiral)

The main search is driven by the table at `0x008224DC..0x00822540` — **25 int32 entries**, each used as a *signed byte* angular offset in 256-unit compass:

| Idx | Offset (bytes/deg) | Idx | Offset |
|---|---|---|---|
| 0 | 0 (0°)      | 13 | +8 (11.25°)  |
| 1 | +1 (1.41°)  | 14 | -8 (-11.25°) |
| 2 | -1          | 15 | +16 (22.5°)  |
| 3 | +2          | 16 | -16          |
| 4 | -2          | 17 | +24 (33.75°) |
| 5 | +3          | 18 | -24          |
| 6 | -3          | 19 | +32 (45°)    |
| 7 | +4          | 20 | -32          |
| 8 | -4          | 21 | +48 (67.5°)  |
| 9 | +5          | 22 | -48          |
| 10| -5          | 23 | +64 (90°)    |
| 11| +6          | 24 | -64          |
| 12| -6          |    |              |

(256 compass units = 360°, so 1 unit ≈ 1.406°.)

The table is a **bidirectional fan expanding outward from dead-ahead**: try straight at the target first, then alternate left/right by 1 compass unit, 2, 3, 4, 5, 6, then jump to 8, 16, 24, 32, 48, 90°. **The fan never exceeds ±90°** — a unit will not try to firing-position *behind* itself relative to the target. This is one of the subtler feel details: it means a tank whose target is blocked from the front will sweep the front semicircle but never consider firing from the back.

The outer loop iterates a **shrinking approach radius** from `radius_leptons` (clamped weapon-range, possibly minus 128) down to `204.8 leptons` (`DAT_007E9238`, 0.8 cells) in steps of **256 leptons** (1 cell):

```c
current_radius = radius_leptons;
while (current_radius > 204.8) {                         // DAT_007E9238
    if (approach_done) break;
    for each of 25 entries in AngleTable[0x8224DC]:
        angle_byte = bearing_byte + (char)AngleTable[i];
        cell_x = target_cell_x + round(current_radius * cos_lookup(angle_byte))
        cell_y = target_cell_y + round(current_radius * sin_lookup(angle_byte))
        candidate_cell = (cell_x, cell_y)

        if (unit is CloseRange infantry):
            candidate_coord = PlaceInfantryInCell(candidate_cell, subcell=1)
        else:
            // cell center in leptons: cell*256 + 128
            candidate_coord = (cell_x*256 + 128, cell_y*256 + 128, Z=0)

        // CloseRange point-blank acceptance: accept if target distance
        // from candidate is under DAT_007E9230 = 307.2 leptons (1.2 cells)
        if (InRange(candidate_coord, TarCom, weapon) OR
            (CloseRange AND target_distance < 307.2)) {

            if (!is_in_playfield(candidate_cell)) continue;

            // Passability check: rebuild a zone query for this cell
            zone = MapClass::GetZoneID(zone_cell, SpeedType, OnBridge_from_this)
            passable = CellRect::CheckCellPassability(candidate_cell)

            if (passable) {
                // If non-infantry, also spin the 8-compass ring around the
                // candidate to see if an adjacent cell passes — see §4.4
                if (this is infantry type 0xF):
                    goto ACCEPT;            // infantry are allowed to stand
                                            // right on the candidate
                else:
                    // Try 8 compass neighbors of candidate (§4.4)
            }
        }
    }
    current_radius -= 256;                   // shrink one cell, retry
}
```

The shrink loop is critical: the fan is tried at the *maximum* effective range first, then at (range − 1 cell), (range − 2 cells), ... down to 0.8 cells. A unit always prefers to stop as far from the target as it can while still firing — which matches the in-game feel of tanks sitting back at weapon-range edge.

### 4.4 8-compass neighbor ring (non-infantry only)

When a unit is NOT infantry type 0xF and a candidate cell passes InRange but fails CheckCellPassability, the function probes the 8 neighboring cells around the candidate:

```c
for (dir = 0; dir < 8; dir++) {
    nx = candidate_cell_x + g_DirectionOffsets[dir*2]      // @ 0x0089F688
    ny = candidate_cell_y + g_DirectionOffsets[dir*2 + 1]  // @ 0x0089F68A

    if (InRange(neighbor, TarCom, weapon) OR
        (CloseRange AND target_distance < 307.2)) {
        zone = MapClass::GetZoneID(neighbor, SpeedType, OnBridge)
        passable = CellRect::CheckCellPassability(neighbor)
        if (passable) { candidate_cell = neighbor; break; }
    }
}
```

This is the authentic "can't stop on that cell, but the cell next to it is fine" behavior that makes tanks look smart when driving around their target. The 8-compass table at `g_DirectionOffsets @ 0x0089F688` is shared with every other system in gamemd.exe (cross-referenced in ASTAR, DRIVE_PROCESS_MOVEMENT, HIGH_BRIDGE, AIRBURST, BUILDINGCLASS_MASTER, AIRCRAFT, and others).

### 4.5 Pathfind-reachability gate

Even after a candidate passes InRange + Passability, one final check runs:

```c
chebyshev_cells = max(abs(candidate.X - target_cell.X),
                      abs(candidate.Y - target_cell.Y))

// Only skipped if wrapper flag uStack_168.byte3 is set (some internal bypass).
path_cells = FUN_0042D170(candidate, target_cell, this,
                          OnBridge_flag, zone_id_from_fc, -1)

if (path_cells > chebyshev_cells + 8) {
    // This candidate is more than 8 cells detour from a straight line.
    // Reject and rebuild a second probe from the unit's current cell.

    // Probe 2: straight-line vs. pathfind from UNIT's cell to candidate.
    my_cell = this->Get_Cell()
    chebyshev_from_me = max(abs(candidate.X - my_cell.X),
                            abs(candidate.Y - my_cell.Y))
    zone2 = this->GetZone(OnBridge)   // vtable +0xBC
    path_from_me = FUN_0042D170(my_cell, candidate, this, zone2, buffer, 0)
    if (path_from_me > chebyshev_from_me + 8) {
        // Still too winding. Skip this candidate.
        continue_outer_shrink;
    }
}
// Accept: ACCEPT label commits the candidate.
approach_done = true;
```

**Magic #2 — the +8 detour tolerance.** The gate tolerates up to 8 cells of extra pathfinding over Chebyshev ("king's move") distance. Any candidate cell whose actual path from the target (or from the unit) is more than 8 cells longer than a straight line is considered unreachable or too winding, and is rejected. Eight cells is a deliberately slack budget so the scan does not repeatedly reject near-candidates blocked by small obstacles like cliff edges or props.

`FUN_0042D170` (Pathfinder-reset distance probe) calls `PathfinderClass::Reset`, then `MapClass::ResolvePathCoord_BridgeAware` to bridge-adjust both endpoints, then a zone precheck, then takes the Chebyshev plus a bridge-boundary offset from `RulesClass+0xBA..+0xBE` (bridge-zone cost table). It returns `0x7FFFFFFF` when zones don't connect — which always exceeds the +8 budget, so unreachable candidates are silently discarded here.

### 4.6 Accept branch (LAB_004D689A)

When a candidate is accepted:

```c
if (!dry_run) {
    chosen_cell = MapClass::Get_CellClass(uStack_16c)
    this->OnArrival(chosen_cell, 1)                  // vtable +0x480 — sets NavCom + mission step
}
final_cell = MapClass::Get_CellClass(uStack_16c)
release_locomotor_ref()                              // if piggyback loco acquired in phase C
return final_cell
```

The chosen cell is returned to the caller. OnArrival is what commits the NavCom and drives the locomotor into that cell on subsequent ticks.

### 4.7 Full-fallback branch (G)

If the spiral completed without setting `approach_done = true`, control reaches the fallback at 0x004D68FC:

* **HunterSeeker** (TypeClass+0xD27) or **engineer-flagged infantry** (mission 15 + HouseClass+0xEC6): skip finesse, just `OnArrival(TarCom_cell, 1)` and return TarCom. HunterSeekers don't do firing positions — they ram.
* **Otherwise:** call `FootClass::Find_Nearby_Passable_Cell @ 0x0056DC20` with a search radius of `this+0x3D4 + this+0x3E0` (two distance fields, clamped to ≤ 32 cells). This function expands in concentric diamond rings around TarCom, collecting up to 24 cells that pass CellRect checks for our SpeedType/zone. It picks the result closest to the current unit (or a random one from that set if the unit's stored anchor is zero). The returned cell is committed via `Set_ArchiveTarget` + `OnArrival`.
* If even the fallback produces no cell: `OnArrival(0, 1)` — transition to Guard/idle with no NavCom.

---

## 5. The threat-scan wrapper (FUN_004D9920, vtable +0x3C4)

This is **not** related to the approach driver beyond sharing a name.

```c
void FootClass::Greatest_Threat(this, threat_flags, scan_origin, enemy_only) {
    if (this->field_0x688 != 0) {
        threat_flags = (threat_flags & ~2) | 1;   // force weapon-range scan (bit 0 set, bit 1 clear)
    }
    TechnoClass::Greatest_Threat(threat_flags, scan_origin, enemy_only);
    if (this->field_0x688 != 0 && result == 0) {
        this->field_0x688 = 0;                    // clear "currently scanning" flag on no-target
    }
}
```

**The `field_0x688` flag** is `ConvoyDisbanded`, consistent with `FOOTCLASS_STRUCT_LAYOUT.md:222`. Resolved: the sole observed writer is `TechnoClass::Clear_Convoy_Chain @ 0x006EC3A0`, which iterates every member of a convoy (linked list via `member+0x5D8`) and sets `member+0x688 = 1`. The wrapper's semantics make sense in that light — a newly-disbanded convoy unit gets **one aggressive weapon-range target scan** on its first tick after the convoy disbands: `threat_flags |= 1; threat_flags &= ~2` forces the `TechnoClass::Greatest_Threat` scan into weapon-range mode (not guard-range), and `clear on empty` means "we gave you one shot to find an enemy; if nothing in weapon range, revert to normal mission behavior." This is the authentic behavior for convoy escorts / hunter-killer teams after their script ends.

Everything else — alliance filtering, Evaluate_Candidate, Calculate_Threat_Score, Scan_Cell_For_Target — lives in **`TechnoClass::Greatest_Threat @ 0x006F8DF0`**, already documented in detail by `ra2-rust-game-docs/TARGET_ACQUISITION_GHIDRA_REPORT.md` §§2–5.

---

## 6. Magic numbers decoded

All constants read directly from gamemd.exe at the documented addresses.

### 6.1 Lepton / subcell thresholds (IEEE 754 doubles at 0x007E9228..0x007E9250)

| Address | Value (leptons) | Cells | Purpose |
|--------|----------------:|------:|---------|
| `0x007E9228` | 281.6 | 1.10 | (not used by this function; consumed by Mission_Attack timer halving per `FOOTCLASS_MISSION_ATTACK`) |
| `0x007E9230` | 307.2 | 1.20 | CloseRange point-blank acceptance distance. A candidate firing cell whose distance to TarCom is under this passes InRange even if the actual weapon range would reject it. |
| `0x007E9238` | 204.8 | 0.80 | Approach-radius shrink **lower bound**. The spiral stops at this radius. Candidates closer than 0.8 cells are never evaluated by the spiral (the CloseRange subcell-snap at §4.2 handles that range). |
| `0x007E9240` | 384.0 | 1.50 | CloseRange subcell-snap threshold. Infantry with CloseRange=yes whose cell-center-to-target distance is ≤ 1.5 cells attempt the immediate `PlaceInfantryInCell` path (§4.2). |
| `0x007E9248` | 332.8 | 1.30 | CloseRange weapon-range clamp. If TypeClass+0x695 is set and the weapon range is less than 1.3 cells, the effective range used by this function is clamped to `0x14C = 332` leptons. |
| `0x007E9250` | 179.2 | 0.70 | (observed adjacent — not referenced by `Greatest_Threat_Scan`. Likely a related Mission_Attack constant.) |

### 6.2 Other lateral constants

* `0x008224DC..0x00822540`: 25-entry angular spiral fan (see §4.3 table).
* `0x0089F688`: `g_DirectionOffsets` — 8 × int16 X-deltas for compass directions 0–7. `0x0089F68A` is the paired Y-delta table (interleaved, 4-byte stride per direction). Populated at runtime by the class static initializer (hence a memory-read at load time returns zeros).
* `DAT_007E9AC0`: `CLSID_WalkLocomotion` CLSID (four-DWORD GUID) used by the piggyback probe to detect walking locomotion.
* `DAT_00818858`: `IID_IPiggyback` GUID used by `QueryInterface` to fetch the piggybacked locomotor.
* `DAT_008B3DF4`: bridge-height offset (populated at runtime by `BridgeListClass` init; 0 pre-load).

### 6.3 Weapon-range bias

* `-128 leptons` (0x80): weapon ranges above `0x1FF` (511) get biased down by 128 so the firing cell sits 0.5 cells *inside* the kill range. Short-range weapons (≤ 2 cells) are not biased.

### 6.4 Pathfind tolerance

* `+8 cells`: the detour budget over Chebyshev distance used by both `FUN_0042D170` gates (§4.5).

### 6.5 RulesClass+0xDF8 (`ApproachTargetResetMultiplier`)

Used in section (D) as `(RulesClass+0xDF8) * approach_radius_leptons`. Resolved: this is **`ApproachTargetResetMultiplier`** from `[General]`, INI default **`1.5`** (rulesmd.ini line 301, rules.ini line 241). The INI comment is the canonical explanation:

> *"The ApproachTarget position should be recalculated if the target is now more than weapon range times this (My approach target picked a spot range 1x away, so if it gets beyond 1.5 I know it is moving and that I will need to refigure where he is.)"*

Cross-verified in `ra2-rust-game-docs/AI_DIFFICULTY_SYSTEM.md:365`.

Behavioral meaning: when a unit has committed to an approach cell (the ArchiveTarget at this+0x5A0 / +0x5A4) and its live TarCom has drifted more than N× the weapon range from that committed spot, the archive is cleared and the approach is recomputed next tick. This is the "target is moving faster than I can catch up" abort.

**Verified: integer multiplication, not FPU.** Disassembly of the multiplication site at `0x004D59E5`:

```asm
004d59df: MOV ECX, dword ptr [0x008871e0]       ; ECX = g_RulesClass_Instance
004d59e5: MOV EDX, dword ptr [ECX + 0xdf8]      ; EDX = *(int*)(Rules + 0xDF8)  — 32-bit integer load
004d59eb: IMUL EDX, dword ptr [ESP + 0x38]      ; EDX *= weapon_range_leptons  (SIGNED INTEGER multiply)
004d59f0: CMP EAX, EDX                          ; compare distance vs product
004d59f2: JLE 0x004d5a07                        ; if distance <= product, don't reset
```

The field is loaded as a 32-bit int (`MOV EDX, dword ptr`) and multiplied with `IMUL`. No `FLD`/`FMUL`/`FISTP` anywhere near this site. So the INI's `1.5` is **truncated to 1 by the ReadInt parser** when stored at +0xDF8, and the runtime multiplier is effectively **1× (not 1.5×)**.

**Parity implication for the Rust port:** multiply by **1**, not 1.5. The INI comment is misleading — the original engine does a plain "distance > weapon_range" check despite what the key's name and comment suggest. If we ever want to honor the INI value, we'd need to read it as a float and multiply in fixed-point (a deliberate divergence from the binary that would change behavior).

Alternative interpretation: `1.5` might be the designer's *intent* that was never realized — the parser shipping with YR rounds down, and nobody caught it. Either way, the shipping binary uses 1.

---

## 7. Struct offsets referenced

All offsets verified against existing struct layout docs (`FOOTCLASS_STRUCT_LAYOUT.md`, `TECHNOCLASS_VTABLE_COMPLETE.md`) or observable usage in the decompilation.

### FootClass / TechnoClass instance fields

| `param_1[i]` | byte offset | Name / purpose |
|---:|---:|---|
| `[0x2B] (0x58)` ? | 0x2B×4 = 0xAC | (read via vtable – ignore) |
| `[0x82 byte]` | 0x82 | **WarpedOutOf / airstrike-in-progress flag** — set while a unit is mid-warp (Chronosphere / Chrono Miner) or mid-airstrike. Cross-referenced in `FOOTCLASS_MISSION_MOVE_GHIDRA_REPORT.md:381` ("+0x82 byte flag... WarpedOutOf"), `FOOTCLASS_PATHFINDING_AND_MOVEMENT.md:516` ("Unit is being warped"), `FIRE_AT_ANALYSIS.md:89/182/341` (airstrike damage modifier, OpenToppedAnim gate). In Greatest_Threat_Scan §3 case (ii), a non-zero +0x82 clears the "allow scatter" flag — a unit currently warping or airstriking must not decide to scatter into another cell. |
| `[0xB9]` | 0x2E4 | Radio / linked flag (set → disables scatter) |
| `[0xAB]` | 0x2AC | NavCom (destination cell pointer) |
| `[0xAD]` | 0x2B4 | TarCom (primary target pointer) |
| `[0x163]` | 0x58C | NavQueue base pointer (array of cell coords) |
| `[0x166]` | 0x598 | NavQueue count |
| `[0x168]` | 0x5A0 | ArchiveTarget slot (secondary/stashed target cell) |
| `[0x169]` | 0x5A4 | ArchiveTarget stored coord pointer |
| `[0x19D]` | 0x674 | Locomotor COM pointer |
| `[0x1B0]` | 0x6C0 | HouseClass (owner) pointer  |
| `[0x1B1]` | 0x6C4 | TypeClass pointer (cached) — dereferenced for TypeClass field reads |
| `+0x688 byte` | 0x688 | "Currently scanning" flag (per wrapper semantics) — **conflicts with FOOTCLASS_STRUCT_LAYOUT's "ConvoyDisbanded" label** (see §9) |
| `+0x68E byte` | 0x68E | HasFoundAutoTarget (used by Mission_Attack, not this function) |
| `+0x3D5 byte` | 0x3D5 | Underground flag (checked for mission-6 scatter condition) |

### TechnoTypeClass fields (via [0x1B1] deref)

| Byte offset | Name | Usage here |
|---:|---|---|
| `+0x695` | CloseRange flag | Gates subcell snap, range clamp, InRange softening (§4.2, §4.3) |
| `+0xD27` | HunterSeeker | Gates fallback-direct-approach (§4.7) |
| `+0xD33` | CanApproachTarget | Gates approach when Can_Fire_At fails (§3A case ii). Default yes; explicit `no` on `[SPY]` (rulesmd.ini:6836). |
| `+0xD34` | CanRecalcApproachTarget | Gates the retarget-abort branch (§3D). |
| `+0xE12` | `DeployToFire` (bool, default no) | Gates building-blocker early-snap (§3B) — UnitTypeClass field. **Unused in vanilla YR:** grep of rulesmd.ini finds zero units with `DeployToFire=yes`, making this branch TS-legacy dead code in standard play. Verified via `MISSION_ENTER_REFINERY_DOCK_GHIDRA_REPORT.md:960` and the rulesmd.ini documentation comment at line 3613: *"The vehicle must deploy before it can fire (def=no)"*. |
| `+0xEC6` | (on HouseClass, not TypeClass) | Gates engineer-direct fallback (§4.7) |

### RulesClass globals

| Byte offset | Purpose |
|---:|---|
| `+0xBA..+0xBE` | Bridge path-cost offsets (used by FUN_0042D170) |
| `+0xDF8` | `ApproachTargetResetMultiplier` — retarget-abort distance multiplier (see §6.5) |
| `+0xF48` | GuardAreaTargetingDelay (used by TechnoClass::InRange for SensorType bonus) |
| `+0xF54` | InRange radio-link range bonus (used by TechnoClass::InRange) |
| `+0xF5C` | InRange lock-flag bonus (used by TechnoClass::InRange) |

### Vtable slots invoked (FootClass, base = `0x007E8C94`)

| Slot | Target | Purpose |
|---:|---|---|
| `+0x2C`  | Get_RTTI_ID | Returns class type (0xF=Infantry, 6=Building, 0xB=Aircraft, 3=Animation, etc.) |
| `+0x48`  | Get_Coord | Returns CoordStruct* (leptons) |
| `+0x4C`  | Get_Cell_Coord | Cell-center coord |
| `+0x50`  | Is_On_Bridge | Bool |
| `+0x54`  | Is_Locomotor_Busy | Bool |
| `+0x78`  | Set_Coord_Direct (locomotor) | Teleport the unit to a coord |
| `+0x84`  | Class_Of | Returns TechnoTypeClass* |
| `+0xBC`  | Get_Zone_For_Self | Zone ID for pathfinder (bridge-aware) |
| `+0x124` | Set_On_Bridge | Toggle bridge status temporarily |
| `+0x168` | Get_Range_Override / Effective_Weapon_Range | Returns leptons; biased by -128 for ranges > 511 |
| `+0x1BC` | Get_Locomotor | Raw loco pointer |
| `+0x2E4` | Get_Target_Coord (aka Likely_Coord) | Preferred firing origin coord |
| `+0x3A8` | Can_Fire_At | Returns FireError code (0 == OK) |
| `+0x3C4` | Greatest_Threat | (this is the wrapper `0x004D9920`) |
| `+0x3C8` | Set_ArchiveTarget | Clears or sets archive target |
| `+0x3F8` | Get_Weapon | Returns WeaponTypeClass* for InRange |
| `+0x400` | Is_Sensor_Type | Bool — gates InRange GuardArea bonus |
| `+0x404` | Get_Sensor_Range | leptons |
| `+0x480` | OnArrival | Commits NavCom + mission transition |
| `+0x53C` | Greatest_Threat_Scan | (this is `0x004D5690`) |

---

## 8. Who calls it — and the vtable +0x53C override map

**Key correction from initial draft:** the two "direct callers" `FUN_00522340` and `FUN_007414E0` are *not* independent callers. They are the **InfantryClass and UnitClass overrides of vtable slot +0x53C**. Only **AircraftClass** uses the base FootClass implementation (`0x004D5690`) directly.

### vtable +0x53C per subclass (read directly from the class vtables)

| Class | vtable base | +0x53C → | Behavior |
|---|---|---|---|
| **FootClass** | `0x007E8C94` | `0x004D5690` (this function) | Base implementation — the 737-line approach driver |
| **AircraftClass** | `0x007E22A4` | `0x004D5690` (inherited from base) | Aircraft uses the base directly |
| **InfantryClass** | `0x007EB058` | `0x00522340` (override) | Preprocessor for mission states 0x1B–0x1E |
| **UnitClass** | `0x007F5C70` | `0x007414E0` (override) | Preprocessor for crushing / can-crush vehicles |

Verified by reading the vtable slots at `0x007E27E0`, `0x007EB594`, `0x007F61AC` (each = `vtable_base + 0x53C`) and the 4-byte pointer stored there.

### InfantryClass override — `FUN_00522340` @ `0x00522340`

**Mission enum values 0x1B–0x1F resolved.** Decoded from `g_MissionNameTable` (array of string pointers; `Mission_Name @ 0x005B3950` indexes into it; table ends at `0x00816D2C`). Reading the last five entries' string pointers and following them:

| Mission ID | String | Purpose |
|---:|---|---|
| 0x1B (27) | **`Paradrop Overfly`** | Aircraft or passenger mid-paradrop flight |
| 0x1C (28) | **`Wait`** | Waiting for a mission-queue dispatch |
| 0x1D (29) | **`Attack Move`** | A-move — advance to destination while engaging targets along the way |
| 0x1E (30) | **`Spyplane Approach`** | Spy plane mission ingress |
| 0x1F (31) | **`Spyplane Overfly`** | Spy plane mission overflight |

**Mission 0x1D "Attack Move" is the standard A-move command** — common in normal YR gameplay, not an edge case. The other three are aircraft-related transport/scout missions; an InfantryClass instance ends up in those states only while it is a passenger or spawned from such a mission context.

```c
undefined4 __fastcall InfantryClass::Greatest_Threat_Scan(InfantryClass* this /* ,char dry_run=ESI */)
{
    if (this->TarCom == NULL) return 0;
    int mission = this->MissionID;    // param_1[0x1b1]
    if (mission not in {Paradrop_Overfly, Wait, Attack_Move, Spyplane_Approach}) {
        // Common path: delegate to base with HARDCODED 0 (dry_run discarded)
        return FootClass::Greatest_Threat_Scan(this, 0);
    }
    // Special path for those four missions:
    this->vtable->Mark(REDRAW);                     // vtable +0x3C
    if (!HouseClass::IsPlayerControl(this->Owner)
         && this->Owner+0xEC8 != 0                   // HouseClass AI flag
         && this->Owner+0x6AC != 0) {                // another HouseClass state flag
        uint32_t opport_weapon = this->Owner+0x6A8;
        bool can_fire = this->Can_Fire_At(TarCom, opport_weapon);   // vtable +0x3A8
        WeaponTypeClass* wpn = this->GetWeapon(opport_weapon);       // vtable +0x3F8
        if (wpn->weapon_ptr != NULL && wpn->weapon_ptr->flag_0x150 == 0) {
            // We have a usable weapon. Two branches:
            if (mission in {0x1B, 0x1C, 0x1D, 0x1E}) {
                // Currently in a passenger/a-move state and cannot fire:
                // promote to mission 0x1F (Spyplane Overfly) as a "generic transport"
                // holding pattern.
                if (!can_fire && !dry_run)
                    this->Assign_Mission_Target_Dest(0x1F /*Spyplane Overfly*/, 0, 0);
                return 0;
            } else if (can_fire && !dry_run) {
                // Normal mission + can fire: retarget via locomotor, commit to Paradrop Overfly
                if (this->Locomotor != NULL)
                    this->Locomotor->Something_0x10();
                if (!this->Locomotor->Something_0x10()) {
                    this->Assign_Mission_Target_Dest(0x1B /*Paradrop Overfly*/, 0, 0);
                    return 0;
                }
                this->Locomotor->Get_Coord();
                this->field_0x6E4 = 1;
                MissionClass::GetMissionTimerEntry();
                Random::RandomRanged(0, 2);
            }
        }
    }
    return 0;
}
```

**Behavioral summary:** for an AI-controlled infantry on Paradrop Overfly / Wait / Attack Move / Spyplane Approach:
* If the infantry has a "reserved" weapon (from an unlabeled house-level slot at HouseClass+0x6A8 — INI key unverified) and that weapon can fire at the current TarCom, commit the unit to an aggressive pursuit mode (Paradrop Overfly).
* If the weapon can't fire, retreat to a holding-pattern mission (Spyplane Overfly).

The Attack Move branch is the one that matters for standard gameplay: it's the per-tick "decide whether to fire at this target while moving" logic. Without this preprocessor, A-moving infantry would never auto-acquire threats.

### UnitClass override — `FUN_007414E0` @ `0x007414E0`

```c
int __thiscall UnitClass::Greatest_Threat_Scan(UnitClass* this, char dry_run)
{
    // AI-only crushing preprocessor.
    // If AI-controlled AND target is crushable (CanBeCrushed/WeaponAbility 0x11)
    //   AND target is close (Rules+0x1728 leptons, "CrushRange") AND can_crush_check passes:
    //     short-circuit to OnArrival(TarCom, 1) — ram it  (guard on dry_run==0)
    //     return TarCom.
    // Else if TypeClass+0xD6A AND our weapon's warhead has special-effect flag (+0xA0->+0x2C0):
    //     OnArrival(TarCom, 1); return TarCom.
    // Else if TypeClass+0xD29 AND /* various crush gates */:
    //     OnArrival(TarCom, 1); return TarCom.
    // Else:
    //     return FootClass::Greatest_Threat_Scan(this, 0);   // HARDCODED 0
}
```

### The canonical indirect caller

* **`FootClass::Mission_Attack`** (Infantry variant `0x0051F3E0`, Unit variant inside the Mission_Handlers table) invokes `this->vtable[+0x53C](0)`. For an Infantry instance this routes to `FUN_00522340`; for a Unit, to `FUN_007414E0`; for an Aircraft, directly to `0x004D5690`. Documented in `FOOTCLASS_MISSION_ATTACK_GHIDRA_REPORT.md`.

### Resolving the dry-run question

Every observed path leads to `FootClass::Greatest_Threat_Scan` being invoked with **`param_2 = 0`**:
1. Mission_Attack passes 0 directly.
2. InfantryClass override at +0x53C hardcodes `0` in its delegation call.
3. UnitClass override at +0x53C hardcodes `0` in its delegation call.
4. AircraftClass inherits the base slot — the direct call from Mission_Attack still passes 0.

**The dry-run path in the body of `0x004D5690` is dead code in the shipping binary.** It would only be exercised if some unseen caller invoked the base function directly (not via vtable) with a non-zero dry_run. No such caller was found in xref analysis.

Note: InfantryClass and UnitClass subclass overrides *do* respect their own dry-run parameter (`unaff_SI` / `param_2`) internally — gating `OnArrival` and mission-assign calls on it. So the *vtable slot as a whole* honours dry-run for the preprocessor phase. It is only the base FootClass spiral-search body that never sees a non-zero value.

### Rust porting implication

The port can safely **omit the dry-run parameter from the base implementation's body** and only plumb it through the InfantryClass/UnitClass preprocessor equivalents (where it gates their mission-assign / OnArrival side effects). This removes ~25% of the conditional clutter in the 737-line function and is not a parity risk.

---

## 9. Conflicts with existing docs

1. **Name misnomer.** `FOOTCLASS_MISSION_HANDLERS_GHIDRA_REPORT.md`, `FOOTCLASS_VTABLE_COMPLETE.md`, and row 19/48–51 of `TARGET_ACQUISITION_GHIDRA_REPORT.md` all describe `0x004D5690` as "Scan for highest-priority threat" / "finds firing position and approaches". **This function performs no threat evaluation.** It requires TarCom already set and only searches for a cell to fire from (or reasons to abandon). The `FOOTCLASS_MISSION_ATTACK_GHIDRA_REPORT.md` §3 correction is the right reading. New code and future reports should treat this function as **`FootClass::Pursue_Firing_Position`** or **`FootClass::Approach_Target`**; the shipping name stays `Greatest_Threat_Scan` for Ghidra symbol compatibility.

2. ~~**+0x688 flag meaning.**~~ **Resolved.** `FOOTCLASS_STRUCT_LAYOUT.md:222` was right — `+0x688` is `ConvoyDisbanded`. The wrapper's semantics (one-shot weapon-range scan, clear on empty) are the *correct* behavior for a newly-disbanded convoy unit reacquiring. See §5 updated.

3. **Subclass override map.** Row 19 of `TARGET_ACQUISITION_GHIDRA_REPORT.md` treats `FootClass::Greatest_Threat_Scan` as a single entry point for mobile-unit targeting. In practice, vtable `+0x53C` is overridden by InfantryClass (`0x00522340`) and UnitClass (`0x007414E0`); only AircraftClass uses the base directly. The subclass overrides each add a preprocessor stage (area-guard mission switching for infantry, crush-ram for units) before delegating to the 737-line base. Future analysis/port work should consider these three distinct invocation paths, not one.

---

## 10. YR activity and TS-legacy check

**Active in YR:** Yes. All call paths are reachable in a standard YR skirmish:
* Mission_Attack is invoked every tick a unit has a TarCom.
* The two direct callers (FUN_00522340, FUN_007414E0) are ground-unit mission handlers executed by regular unit AI, not SpecialFlags-gated TS code.
* `CLSID_WalkLocomotion` is used by every YR infantry type, so the piggyback probe is live.

**No TS-only gates:** no `SpecialFlags & 0x1000` references, no `FogOfWar`-style fallbacks. The only field that carries a TS-era name is `+0x695 CloseRange` on infantry TypeClass, but it is live in YR (Tanya, Yuri, Boris, demolition infantry all set it).

**TS ghost watch — things that are NOT active:**
* No call into `FUN_00487C10` outside the infantry-building-blocker branch (§3B). That helper itself checks for any building (type 6) in the cell's occupant list — all ordinary cells pass.
* No references to `g_SubterraneanList` or `UndergroundClass` here. The +0x3D5 byte check in §3 case (ii) (`*(char *)((int)param_1 + 0x3D5)`) is a runtime state byte, not a TS-legacy feature gate.

---

## 11. Current Rust implementation status

Per the Rust-scan agent (summarized in §2 of the brainstorm):

| Behavior | Rust status | Notes |
|---|---|---|
| Approach/firing-position search for ground units | **Not implemented** | Combat system only fires at explicitly-set `attack_target`. No spiral fan, no pathfind gate, no subcell snap. Units either fire in-place or don't. |
| Angular fan around bearing (25-entry table) | Missing | The spiral feel is the single biggest visible parity gap — YR tanks noticeably "shuffle" into firing positions; the port does not. |
| CloseRange point-blank teleport (Tanya / Yuri / SEAL) | Missing | Relevant once these units are implemented. |
| `FootClass::Greatest_Threat` wrapper (TechnoClass delegate) | Partial — garrison-style scan in [src/sim/combat/mod.rs:624-758](../src/sim/combat/mod.rs#L624) and [src/sim/combat/combat_targeting.rs::acquire_best_target_for_entity](../src/sim/combat/combat_targeting.rs#L67) | These implement *threat scanning* (the TechnoClass side), not approach. Threat ranking uses distance + armed/unarmed class only; `ThreatPosed` / `SpecialThreatValue` / coefficients are not parsed. |
| Weapon-range -128 bias on firing-position | Missing | Will affect tank stopping distance; currently tanks likely stop at exact range edge. |
| +8-cell detour tolerance gate | Missing (no firing-pos path at all) | |
| Fallback `Find_Nearby_Passable_Cell` | Missing | |
| Approach-target retarget abort (Rules+0xDF8 × range check) | Missing | Gated by `CanRecalcApproachTarget` on the target's TypeClass, not OpportunityFire. |
| HunterSeeker ram-direct behavior | Missing | HunterSeeker not implemented. |

This function is in the path of every attack a unit makes; implementing it is a prerequisite for faithful combat feel (tanks shuffling into range, melee snap, oppfire behavior). The Mission_Attack dispatch in `FOOTCLASS_MISSION_ATTACK_GHIDRA_REPORT.md` §4 is the outer loop; `Greatest_Threat_Scan` is what it delegates to each tick.

---

## 12. Open questions

All seven original open questions are now resolved.

1. ~~RulesClass+0xDF8 INI mapping.~~ **Resolved:** `ApproachTargetResetMultiplier` from `[General]`. INI says `1.5`, effective runtime value is **`1`** due to integer storage. See §6.5.
2. ~~TypeClass+0xE12 INI mapping.~~ **Resolved:** `DeployToFire` (bool, default no) on UnitTypeClass. See §7 TypeClass table. No vanilla YR unit sets this, so the §3B early-snap branch is **TS-legacy dead code** in standard YR play.
3. ~~Field `+0x688` label collision.~~ **Resolved:** `ConvoyDisbanded`, written by `TechnoClass::Clear_Convoy_Chain @ 0x006EC3A0`. See §5 and §9 item 2.
4. ~~Whether any indirect caller ever passes `param_2 != 0`.~~ **Resolved:** No. See §8 "Resolving the dry-run question."
5. ~~Field `+0x82` byte flag.~~ **Resolved:** `InOpenToppedTransport` (canonical name — verified via sole writer `TechnoClass::SetInOpenTransport @ 0x00710470`, which sets +0x82=1 then calls vtable+0x3D0 (Hide) and the cell-removal helper). The chrono-warp and airstrike-delivery code paths overload the **same byte** as an "in transit / contained" marker — there is no separate WarpedOutOf field. See `BULLETCLASS_LIFECYCLE_AND_TIER1_VERIFICATIONS_GHIDRA_REPORT.md` §1.5 for the writer-site verification.
6. ~~Sub-mission enum values 0x1B–0x1E.~~ **Resolved** via `g_MissionNameTable @ 0x00816CAC..0x00816D2C`:
   - 0x1B = `Paradrop Overfly`
   - 0x1C = `Wait`
   - 0x1D = **`Attack Move`** (the standard A-move mission — NOT an edge case)
   - 0x1E = `Spyplane Approach`
   - 0x1F = `Spyplane Overfly` (referenced as the fall-back "holding pattern")

   See §8 InfantryClass override section.
7. ~~Ghidra decompilation of `ApproachTargetResetMultiplier` as an int.~~ **Resolved:** integer. Disassembly at `0x004D59E5` uses `MOV EDX, [rules+0xDF8]` followed by `IMUL EDX, weapon_range`. No FPU. The INI value `1.5` is truncated to `1` during parse. See §6.5.

## 13. Parity notes for the Rust port

Collected from the resolutions above — things the port MUST do to match the binary's behavior, not the INI's documentation:

1. **Multiply `ApproachTargetResetMultiplier` by 1, not 1.5.** The INI comment is misleading; the shipping binary uses integer storage and the value truncates.
2. **Skip the §3B building-blocker early-snap branch** in a first-pass implementation. No vanilla YR unit has `DeployToFire=yes`. Correctness-wise, implementing it would be a no-op; skipping it saves complexity and is a verified parity match for every stock unit.
3. **Implement the ConvoyDisbanded one-shot aggressive scan** (§5) for units that were in a convoy that was just cleared. The semantics are "one weapon-range-mode scan and then fall back to normal."
4. **Implement the InfantryClass A-move preprocessor** (§8) — mission 0x1D (Attack Move) needs the per-tick Can_Fire_At + retarget-to-Paradrop-Overfly / promote-to-Spyplane-Overfly dispatch for AI-controlled infantry. Without this, A-move infantry will never auto-engage.
5. **Omit the dry_run parameter from the base function body.** Carry dry_run only through the InfantryClass/UnitClass preprocessor equivalents where it gates mission-assignment side effects.
6. **InOpenToppedTransport (+0x82) blocks scatter.** Canonical name — chrono-warp and airstrike delivery overload this byte as a transit/contained marker (verified via `TechnoClass::SetInOpenTransport @ 0x00710470`). When implementing Chronosphere-teleport and airstrikes, set this flag during transit so the unit does not try to scatter mid-warp. Clear on arrival.

---

## Sources

**Ghidra decompiled:**
* `0x004D5690` — FootClass::Greatest_Threat_Scan (full, 737 lines)
* `0x004D9920` — FootClass::Greatest_Threat wrapper
* `0x00522340` — InfantryClass Mission_Hunt-like caller (direct caller #1)
* `0x007414E0` — UnitClass Mission_Hunt-like caller with crush preprocessor (direct caller #2)
* `0x00487C10` — IsCellValidForInfantryApproach (no-building, terrain-passable)
* `0x00486900` — Mission-state double-check
* `0x0042D170` — Pathfinder-reset distance probe
* `0x005F6360` — 3D-distance-with-foundation-adjust
* `0x0045A050` — QueryInterface helper
* `0x0070FEB0` — Is_In_Attack_State helper (reads this+0x1CC)
* `0x0056DC20` — FootClass::Find_Nearby_Passable_Cell
* `0x006F7220` — TechnoClass::InRange (for InRange semantics)

**Memory reads (gamemd.exe):**
* `0x008224DC..0x00822540`: 25-entry angular spiral fan (100 bytes)
* `0x007E9228..0x007E9250`: 6 IEEE 754 doubles (approach thresholds)

**Docs referenced:**
* `C:/Users/enok/Documents/ra2-rust-game-docs/TARGET_ACQUISITION_GHIDRA_REPORT.md` — parent threat-system report
* `C:/Users/enok/Documents/ra2-rust-game-docs/FOOTCLASS_MISSION_ATTACK_GHIDRA_REPORT.md` — Mission_Attack dispatch + §3 partial sketch of this function
* `C:/Users/enok/Documents/ra2-rust-game-docs/FOOTCLASS_VTABLE_COMPLETE.md` — vtable slot identities
* `C:/Users/enok/Documents/ra2-rust-game-docs/FOOTCLASS_MISSION_HANDLERS_GHIDRA_REPORT.md` — vtable dispatch summary
* `C:/Users/enok/Documents/ra2-rust-game-docs/FOOTCLASS_STRUCT_LAYOUT.md` — instance field offsets (source of the +0x688 label conflict)
* `C:/Users/enok/Documents/ra2-rust-game-docs/FOOTCLASS_PATHFINDING_AND_MOVEMENT.md` — g_DirectionOffsets reference
* `C:/Users/enok/Documents/ra2-rust-game-docs/HIGH_BRIDGE_DAMAGE_STATE_MACHINE_GHIDRA_REPORT.md` §11.7 — g_DirectionOffsets layout

**INI:**
* `c:/Users/enok/Documents/ra2-rust-game/ini/rulesmd.ini`: `CloseRange=`, `GuardRange=`, `DefaultToGuardArea=`, `HunterSeeker=`, `VirtualScanner`, `CanApproachTarget=`, `MyEffectivenessCoefficient*` family. Most feed `TechnoClass::Greatest_Threat`, not this function. `OpportunityFire=` is read from the same INI but lives at `TypeClass+0x6AF` and does NOT interact with this function — see `OPPORTUNITY_FIRE_GHIDRA_REPORT.md`.

**Rust source:**
* [src/sim/combat/combat_targeting.rs](../src/sim/combat/combat_targeting.rs) — current threat scanning implementation
* [src/sim/combat/mod.rs](../src/sim/combat/mod.rs) — garrison-style auto-acquire
