# Zone System Fidelity Fixes — TODO

All findings verified against gamemd.exe binary via Ghidra MCP.
Binary passability matrix read from 0x82A594 (416 bytes = 13×8×4).
RecalcZoneType decompiled at 0x483C80. TMP→LandType table read from 0x8288E4.

## FINDING 1 — Row 5 (Amphibious) has wrong values
**Confidence:** VERIFIED — binary read_memory confirms
**File:** `src/sim/pathfinding/passability.rs` line 117
**Impact:** CRITICAL — amphibious units can't zone-reach water/beach cells

Binary row 5: `[1, 2, 2, 1, 1, 2, 2, 3]` — Ground + Beach + Water passable
Rust row 5:   `[1, 2, 2, 2, 2, 2, 1, 3]` — Ground + Railroad passable (Subterranean's profile)

Differences at columns 3 (Beach: should be 1), 4 (Water: should be 1), 6 (should be 2).
All other 12 rows are byte-perfect matches.

**Cause:** MovementZone enum fixed (Amphibious inserted at index 5) but matrix row not updated.

**Fix:** Replace row 5 with `[1, 2, 2, 1, 1, 2, 2, 3]`.

## FINDING 2 — Column semantics mismatch (Rough/Railroad/Rock terrain)
**Confidence:** VERIFIED — RecalcZoneType (0x483C80) decompiled + TMP table (0x8288E4) read
**File:** `src/sim/pathfinding/passability.rs` (matrix + `tmp_terrain_to_land_type`)
**Impact:** CRITICAL — Normal/Crusher units blocked on rough/railroad terrain; Fly/Subterranean blocked on rock

### The mapping chain differs

**Original engine (3-step):**
```
TMP byte → LandType (0-10+) via DAT_008288e4
LandType → ZoneType (0-7) via RecalcZoneType
ZoneType → passability via matrix[MovementZone][ZoneType]
```

**Our engine (2-step):**
```
TMP byte → LandType (0-7) via tmp_terrain_to_land_type
LandType → passability via matrix[MovementZone][LandType]
```

### How RecalcZoneType actually maps terrain (from binary decompilation)

```
If not in playfield → ZoneType 7 (OoB)
If overlay inherited ObjectTypeClass+0x22D Crushable=yes → ZoneType 1
If wall overlay (IsWall / OverlayType+0x2A8) → ZoneType 2
If overlay Wheel speed == 0 or IsARock → ZoneType 6 (Impassable)
If tiberium overlay (IsTiberium) → ZoneType 6 (Impassable)
If overlay IsRubble → ZoneType 0 (Ground)
If LandType == 2 (Water) → ZoneType 4 (Water)
If LandType == 6 (Beach) → ZoneType 3 (Beach)
If speed[LandType] <= 0 → ZoneType 6 (Impassable)
If building on cell → ZoneType 5 or 6
Default → ZoneType 0 (Ground)  ← MOST terrain types fall here
```

Key: Rough (orig LT 7), Railroad (orig LT 9), Ice (orig LT 8), and Road terrain
without overlay (orig LT 1) ALL fall through to **ZoneType 0 (Ground)**. Only
Water, Beach, overlays (Crushable/Wall/Tiberium/impassable), buildings, and speed=0 terrain get non-zero
ZoneTypes.

### Specific mismatches

| TMP byte | Terrain | Original ZoneType | Our LandType col | Result |
|---|---|---|---|---|
| 14 | Rough | 0 (Ground) → col 0 | Rough → col 2 | **Normal blocked on Rough (should pass)** |
| 5-6 | Tunnel/Railroad | 0 (Ground) → col 0 | Railroad → col 6 | **Normal blocked on Railroad (should pass)** |
| 7-8,15 | Rock/Cliff | 6 (Impassable) → col 6 | Rock → col 7 | **Fly/Subterranean blocked (should pass)** |
| 11-12 | Road (TMP) | 0 (Ground) → col 0 | Road → col 1 | **Normal blocked on road terrain w/o overlay** |

**Binary col 6 (Impassable) values:** `[2,2,2,2,2,2,1,2,2,1,2,2,2]`
→ Subterranean(row 6)=1 and Fly(row 9)=1 CAN enter Impassable cells.

**Our col 7 (Rock) values:** `[3,3,3,3,3,3,3,3,3,3,3,3,3]`
→ NOTHING can enter Rock. Missing Subterranean/Fly exception.

### Concrete scenario (verified)

**Rhino Tank (MovementZone=Normal, SpeedType=Track) pathfinding across Rough terrain:**

Original:
```
Clear cell → ZoneType 0, zone_id=1
Rough cell → ZoneType 0, zone_id=1 (same zone, both Ground)
can_reach → same zone → YES
```

Our engine:
```
Clear cell → LandType 0 (Clear), matrix[0][0]=1 → zone_id=1
Rough cell → LandType 2 (Rough), matrix[0][2]=2 → ZONE_INVALID
can_reach → destination ZONE_INVALID → FALSE
```
**Player sees: tank refuses to move across rough terrain patches.**

### Fix options

**Option A (minimal — recommended first):** Fix `tmp_terrain_to_land_type` to map
Rough→Clear(0) and Railroad→Clear(0), since original treats them as Ground.
Then fix Rock to map to a new "Impassable" column with binary col 6 values
(passable for Subterranean and Fly).

**Option B (structural):** Add a RecalcZoneType-equivalent function that takes our
richer LandType + overlay info and produces the original 8 ZoneType values.
Use ZoneType as the matrix column index. Most faithful to original architecture.

## FINDING 3 — Zone connection graph not used by Can_Reach_Zone (CORRECTED)
**Confidence:** VERIFIED — Can_Reach_Zone (0x56D100) + zone_id allocation in UpdateBridgeZonesHelper
**File:** `src/sim/pathfinding/zone_build.rs`, `zone_map.rs`
**Impact:** LOW (corrected from HIGH) — basic reachability behaves equivalently

**CORRECTION:** Previous analysis claimed Can_Reach_Zone uses the zone connection
graph (MapClass+0x14). This is WRONG. Decompilation of Can_Reach_Zone (0x56D100)
shows it simply does:
```
zone_id_src = GetZoneID(src, movementZone, ...)
zone_id_dst = GetZoneID(dst, movementZone, ...)
return zone_id_src == zone_id_dst
```
No graph traversal. Just zone_id equality.

**Zone_id allocation in original (from UpdateBridgeZonesHelper at 0x56C510):**
- Passable clusters → assigned zone_ids 2, 3, 4, ... (one per connected component)
- ALL blocked clusters → zone_id = 1 (shared, never overwritten)
- Cluster 0 → 0xFFFF (sentinel)

**Comparison for "source passable, destination blocked":**
- Original: zone_id_src (2+) != zone_id_dst (1) → false
- Our code: zone_id_src (1+) != ZONE_INVALID (0) → false
- **Same result.** ✓

**The zone connection graph IS used by Zone_precheck (0x42C290)** — the hierarchical
(corrected 2026-05-29: was 0x42C339; binary entry is 0x42C290, body 0x42C290–0x42C8F7; 0x42C339 is a mid-function address — via get_function_by_address 0x42C339 returning entry 0x42C290 — ROOT_CAUSE: GHIDRA_ADDRESS_SHIFT)
pathfinder for route planning through zone graphs. We don't have this system, but
it's a performance optimization for long-range pathfinding, not a correctness issue
for basic reachability.

**Remaining structural difference:** Original groups blocked clusters separately
(all get zone_id=1), so two blocked cells report as "mutually reachable." Our code
gives both ZONE_INVALID, so they're mutually unreachable. This only matters if a
unit is somehow ON a blocked cell — unlikely in practice.

## FINDING 4 — Only 6 zone maps instead of 13 per-MovementZone
**Confidence:** VERIFIED — MapClass+0x18 contains 13 zone_ids pointers
**File:** `src/sim/pathfinding/zone_map.rs`
**Impact:** MEDIUM — units with different passability profiles share zones

Original: 13 separate zone_ids arrays (one per MovementZone).
Ours: 6 ZoneCategory maps using representative_movement_zone.

Crusher and Normal share Land category (representative=Normal). Crusher allows Road
cells but the shared zone map uses Normal's row which blocks Road.

## FINDING 5 — SpeedType enum order wrong
**Confidence:** VERIFIED — from ZONE_PASSABILITY_VERIFIED.md + binary INI parser
**File:** `src/rules/locomotor_type.rs`
**Impact:** LOW (parsed by name, not index) — would break lockstep serialization

Binary: Foot(0),Track(1),Wheel(2),Hover(3),Winged(4),Float(5),Amphibious(6),FloatBeach(7)
Rust:   Foot(0),Track(1),Wheel(2),Float(3),Amphibious(4),Winged(5),FloatBeach(6),Hover(7)

## FINDING 6 — No diamond playfield test
**Confidence:** VERIFIED — Is_Cell_In_Playfield at 0x578460
**Impact:** LOW — zones computed for cells outside diamond; wastes cycles, doesn't affect gameplay

## FINDING 7 — Flood-fill height threshold asymmetry
**Confidence:** VERIFIED — ZoneFloodFillScanLine (0x56CB90) decompiled
**File:** `src/sim/pathfinding/zone_build.rs` line 288
**Impact:** LOW — subtle zone splitting at steep terrain transitions

Original scan-line flood-fill uses different height thresholds:
- Left scan: abs(consecutive_height_diff) ≤ 1
- **Right scan: abs(consecutive_height_diff) ≤ 3**
- Recursive scans: abs(diff) ≤ 1

Our BFS uses abs(diff) > 1 for ALL directions, matching the strictest threshold.
The rightward scan's leniency (≤ 3) may handle bridge approaches or steep ramps.
Our code would split zones at those transitions.

## FINDING 8 — Speed threshold confirmed as 0.01
**Confidence:** VERIFIED — `FCOMP double ptr [0x7E3808]`, double value = 0.01
**Impact:** Informational

RecalcZoneType step 9: `if speed[LandType*9] <= 0.01 → ZoneType 6 (Impassable)`.
Only terrain with speed ≤ 1% is classified as impassable. Rough terrain (~50-70% speed),
Railroad (positive speed), and other passable terrain safely fall through to Ground.

Step 2c (overlay check) uses exact `== 0.0` comparison (verified from assembly:
`TEST AH,0x40` checks C3 only = equality).

## STALE DOC

**ZONE_PASSABILITY_VERIFIED.md** claim "MovementZone enum missing Amphibious (index 5)"
is outdated — enum now has all 13 variants. However, the matrix row 5 values were not
updated (Finding 1), so the underlying issue persists in different form.
