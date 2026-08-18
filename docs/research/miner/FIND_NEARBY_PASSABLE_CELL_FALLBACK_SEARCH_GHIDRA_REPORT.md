# Find Nearby Passable Cell Fallback Search - Ghidra Research Report

**Address(es):** `0x0073E5E0` (`UnitClass__Mission_Harvest`), `0x0056DC20` (`FootClass__Find_Nearby_Passable_Cell`), `0x0056E7C0` (`CellRect__CheckPassability`), `0x00586780` (`CellRect__CheckOccupancy`)  
**Investigation Mode:** exhaustive-slice  
**Claimed Scope:** exact arguments, search order, effective radius/bounds, and result selection when Chrono Miner `Mission_Harvest` state 2 far-return fallback calls `Find_Nearby_Passable_Cell`.  
**Non-Scope:** full pathfinding, A*, normal close radio dock path, post-arrival docking/radio messages, post-unload exit, and all other callers of `Find_Nearby_Passable_Cell`.  
**Confidence:** High for scoped callsite and helper control flow; Medium for human names of some helper parameters inherited from prior validator reports.  
**Active in YR:** Yes. Stock `[CMIN]` has `Harvester=yes`, `Teleporter=yes`, and `Dock=NAREFN,GAREFN`; stock `[GAREFN]`/`[NAREFN]` have `QueueingCell=4,1`; the far fallback is reached when the state-2 close reservation/radio path does not fire.

## Target Question

What exact search order, radius/bounds, and inputs are used when the Chrono Miner far-return fallback reaches `Find_Nearby_Passable_Cell`?

## Non-Goals

- Do not investigate all pathfinding or A*.
- Do not re-open settled close-path dock-anchor findings.
- Do not infer post-arrival docking behavior from this fallback cell.
- Do not edit Rust or unrelated research docs in this slot.

## Evidence Needed To Mark COMPLETE

- Callsite stack/ECX evidence for every `Find_Nearby_Passable_Cell` argument.
- Helper-body evidence for outer radius, candidate coordinate order, stop conditions, and final selection.
- YR liveness evidence from standard CMIN/refinery INI flags plus live `Mission_Harvest` branch.
- Rust-facing delta and acceptance test handoff.

## Stop Conditions

- Stop after the Chrono Miner state-2 fallback call and the helper behavior needed to rank its returned cell.
- Stop before A*, locomotor movement after `Set_Destination`, or unrelated `Find_Nearby_Passable_Cell` callers.
- If a Ghidra function boundary is missing, record uncertainty instead of creating functions. No mutation was needed.

## 1. Overview

Chrono Miner far-return fallback seeds the search at `refinery_anchor + QueueingCell` and calls `Find_Nearby_Passable_Cell` with a 1x1 passability probe, zone disabled, no explicit occupancy-rect check, null target, and `skip_first_quad=0`. The helper scans square perimeter rings in cell coordinates, collects up to 24 accepted candidates, stops after the first ring containing at least one direct candidate, then selects by `g_CurrentFrameCounter % count` because the target argument is `{0,0}`.

The pushed literal `2` at the callsite is `speed_type`, not a search radius and not a 2x2 footprint. The effective ring limit comes from the helper receiver's `+0xF4 + +0xF8`, capped at `0x20`; in this call ECX is `0x0087F7E8`, the `MapClass` singleton, so this is effectively `min(MapClass.Size.width + MapClass.Size.height, 32)`, not CMIN speed/sight.

## 2. Scoped Inputs And Offsets

| Item | Verified value in CMIN fallback | Evidence | Active in YR |
|---|---|---|---|
| Receiver / ECX | `0x0087F7E8` (`MapClass` singleton) | assembly before `0x0073ED75`: `MOV ECX,0x87f7e8`; same singleton used for `MapClass__Get_CellClass @ 0x005657A0` | Yes |
| output cell arg | stack pointer to local output cell | pushes before `CALL 0x0056DC20`; helper writes `*param_2` | Yes |
| origin cell arg | `refinery_anchor + BuildingType.QueueingCell` | `0x0073ED25` reads `+0x1618`, `0x0073ED34` reads `+0x161C`, call at `0x0073ED75` | Yes |
| `speed_type` | `2` | `0x0073ED5E PUSH 0x2` as third stack arg after origin/output | Yes |
| zone id | `-1` | `0x0073ED58 PUSH -0x1`; helper also maps `0xFFFF` to `-1` | Yes |
| movement zone / locomotor arg | `0` | `0x0073ED56 PUSH ESI`, `ESI=0` | Yes |
| bridge-aware height arg | `0` | `0x0073ED57 PUSH ESI` | Yes |
| foundation width / height | `1,1` | `0x0073ED55 PUSH EAX`, `0x0073ED54 PUSH ECX`, both loaded from `EAX=1` | Yes |
| reject-any-overlay | `0` | `0x0073ED53 PUSH ESI` | Yes |
| check height match | `0` | `0x0073ED52 PUSH ESI` | Yes |
| obstacle-free object-list check | `0` | `0x0073ED4F PUSH ESI` | Yes |
| bridge cell allow/reject flag | `1` (bridge cells allowed under helper's inverted test) | `0x0073ED4E PUSH EAX`; helper rejects bridge only when this param is zero | Yes |
| target cell | `{0,0}` null target | `0x0073ED6B/70` zero local target words; `0x0073ED4D PUSH EDX` | Yes |
| skip-first-quadrant | `0` | `0x0073ED4C PUSH ESI` | Yes |
| check occupancy rect | `0` | `0x0073ED42 PUSH ESI`; no `CellRect__CheckOccupancy` call can fire for this CMIN fallback | Yes |

## 3. Core Logic

### 3.1 Effective Radius And Bounds

Verified behavior: `Find_Nearby_Passable_Cell` computes:

```text
limit = receiver[+0xF4] + receiver[+0xF8]
if limit > 32: limit = 32
if limit <= 0: return NullCell
for ring = 0; ring < limit; ring++:
    scan ring
```

Evidence: `0x0056DCE1` reads `[EBX+0xF8]`, `0x0056DCE7` reads `[EBX+0xF4]`, `0x0056DCEF..0x0056DCF4` caps at `0x20`, and the loop exits when incremented `ring >= limit`.

For this Chrono Miner call, `EBX` was copied from ECX, and ECX was `0x0087F7E8` at `0x0073ED66`. Prior MapClass reports identify `MapClass+0xF4/+0xF8` as map `Size` width/height. Therefore standard maps with width+height over 32 search rings `0..31`. There is no caller-provided radius argument of `2` in this path.

Active in YR: Yes. The call is in live `UnitClass__Mission_Harvest` state 2 for stock CMIN fallback.

### 3.2 Candidate Coordinate Order

The scan is a square perimeter in cell coordinates, not a diamond in `(x,y)` cell space. It may render like an isometric diamond on screen, but the binary coordinate order is:

```text
for ring r = 0 .. limit-1:
  for d = -r .. +r:
    if skip_first_quadrant == 0:
      test (ox + d, oy - r)       ; top row, west to east
    if skip_first_quadrant == 0 || -r < d:
      test (ox + d, oy + r)       ; bottom row, west to east

  for d = 1-r .. r-1:
    if skip_first_quadrant == 0:
      test (ox - r, oy + d)       ; left column, north to south excluding corners
    test (ox + r, oy + d)         ; right column, north to south excluding corners
```

For the Chrono Miner fallback `skip_first_quadrant=0`, so all four tests are active. For `r=0`, the origin is tested twice: once as top row and once as bottom row. If both validations pass, the same packed cell can be collected twice; final modulo selection still returns the same coordinate.

Evidence: first row pair at `0x0056DD14..0x0056E141`, left/right column pair at `0x0056E159..0x0056E578`; loop bounds use `iVar14=-r..r` and `iVar14=1-r..r-1`.

Active in YR: Yes.

### 3.3 Per-Candidate Validation In This Fallback

With the CMIN arguments, each candidate must pass:

1. Fixed 512-wide cell lookup with dummy-cell fallback for out-of-range/null cell pointers.
2. `TechnoClass__IsOnScreen(cell, 1)`.
3. `CellRect__CheckPassability(candidate, width=1, height=1, speed_type=2, zone=-1, movement_zone=0, required_height=-1, bridge_aware=0, reject_any_overlay=0)`.
4. Bridge rejection is skipped because the bridge flag argument is `1`.

The following checks are disabled in this CMIN fallback: helper height-match check, `TechnoClass__Is_Current_Cell_Obstacle_Free`, `CellRect__CheckOccupancy`, target-distance selection, and overlay-any rejection.

Evidence: callsite args at `0x0073ED42..0x0073ED75`; validation branches in `0x0056DC20`; validator contracts from `0x0056E7C0` and `0x00586780`.

Active in YR: Yes.

### 3.4 Candidate Cap, Early Stop, And Selection

The helper stores accepted cells in a 24-entry local array and jumps to selection once count reaches `0x18`. If at least one direct candidate was found, it completes the current ring and stops before scanning farther rings.

For `param_7=0` in this CMIN fallback, an accepted candidate is "direct" only if `FUN_006D6410(candidate_center)` resolves back to the same cell. Indirect candidates are still collected, but they do not set the early-stop flag.

At selection, every collected candidate is classified again into direct and indirect arrays. Because the CMIN fallback target argument is `{0,0}`, the final pick is:

```text
if direct_count > 0:
    result = direct_candidates[g_CurrentFrameCounter % direct_count]
else:
    result = indirect_candidates[g_CurrentFrameCounter % indirect_count]
```

The decompiler renders direct-array access as `local_60[index - 0x18]`, but stack layout makes that the adjacent direct-candidate array, consistent with the prior full helper report.

Evidence: cap check `local_1d4 == 0x18`; early-stop flag `local_1d5`; selection block `0x0056E5B3..0x0056E79A`.

Active in YR: Yes.

## 4. INI Keys

| INI key | Stock value | Role in this slice | Evidence | Active in YR |
|---|---|---|---|---|
| `[CMIN] Harvester` | `yes` | reaches harvester mission state machine | `ini/rulesmd.ini:7364`; `0x0073E5E0` type flag gate | Yes |
| `[CMIN] Teleporter` | `yes` | selects chrono branch and makes fallback destination branch enter once a fallback dock is found | `ini/rulesmd.ini:7396`; `0x0073E5E0` `Type+0xCD4` | Yes |
| `[CMIN] Dock` | `NAREFN,GAREFN` | supplies dock-list searched before fallback cell selection | `ini/rulesmd.ini:7361` | Yes |
| `[GAREFN]/[NAREFN] QueueingCell` | `4,1` | origin seed for this helper call | `ini/artmd.ini:1773`, `1716`; binary reads `BuildingType+0x1618/+0x161C` | Yes |
| `[General] ChronoHarvTooFarDistance` | `50` | controls whether this fallback is reached; not an argument to the helper | `ini/rulesmd.ini:294`; `0x0073E5E0` close-path compare | Conditional |
| `[Map] Size` | map-specific | effective helper ring limit in this call is capped width+height | MapClass reports identify `+0xF4/+0xF8`; `0x0073ED66` passes MapClass singleton | Yes |

## 5. Integration Points

`UnitClass__Mission_Harvest @ 0x0073E5E0` computes the seed, calls `Find_Nearby_Passable_Cell`, compares the returned packed cell with null-cell sentinel `DAT_00B1CFB8`, then either clears destination or converts the packed cell through `MapClass__Get_CellClass @ 0x005657A0` before vtable `+0x480` `Set_Destination`.

Active in YR: Yes. Stock CMIN and refinery definitions reach this path when close radio reservation does not fire.

## 6. Current Rust Implementation Status

Current Rust has a nearby-cell helper for miner docking, but this scoped binary slice shows mismatches/unchecked areas:

| Rust surface | Observed status | Delta |
|---|---|---|
| `src/sim/miner/miner_system.rs:1052` `chrono_return_staging_cell_for_sid` | seeds from `QueueingCell` and calls `find_nearby_passable_cell_with_index` with `sim.tick` | correct seed concept; selection index source is plausible but not verified equivalent to `g_CurrentFrameCounter` in this report |
| `src/sim/miner/miner_dock_sequence.rs:39` | `EXIT_SEARCH_MAX_RADIUS = 16` | mismatch for CMIN fallback: binary effective cap is normally 32 rings (`0..31`) from MapClass size cap |
| `src/sim/miner/miner_dock_sequence.rs:268` | collects all passable cells in first non-empty ring and modulo-picks | partial: binary caps total candidates at 24, separates direct vs indirect, and can continue past indirect-only rings |
| `src/sim/miner/miner_dock_sequence.rs:276` | ring 0 returns a single origin candidate | behavior-equivalent if origin is passable, but binary tests/collects origin twice |
| `src/sim/miner/miner_dock_sequence.rs:282` | scans top/bottom then left/right square perimeter | order matches CMIN `skip_first_quadrant=0` ring order, despite comments saying "diamond-ring" |
| `src/sim/pathfinding/core.rs:1199` | generic nearest-walkable helper checks first passable and includes corner duplicates in left/right candidate group | not the helper used by current CMIN staging path, but not binary-equivalent for this selection behavior |

No Rust files were edited.

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| CMIN fallback `Find_Nearby` call args | verified | assembly `0x0073ED42..0x0073ED75` | none |
| Seed formula into helper | verified from prior reports and spot-check | `0x0073ED25`, `0x0073ED34`; starting reports | none |
| Effective radius for this call | verified | `0x0073ED66`, `0x0056DCE1..0x0056DCF9`; MapClass reports | none |
| Ring coordinate order | verified | `0x0056DD14..0x0056E578` | none |
| Candidate cap and stop conditions | verified | `0x0056DF2A`, `0x0056E141`, `0x0056E383`, `0x0056E578`, `0x0056E5B3` | none |
| Direct/indirect selection with null target | verified | `0x0056E5B3..0x0056E79A` | exact `FUN_006D6410` internals not re-opened; prior helper report used |
| `CheckPassability` contract for passed args | verified by cross-doc and decompile | `0x0056E7C0`, `0x004834A0` | full terrain matrix values out-of-scope |
| `CheckOccupancy` in CMIN fallback | verified disabled | call arg `param_16=0`; helper branches | none |
| YR liveness | verified | stock INI and live `Mission_Harvest` branch | runtime scenario to force fallback remains a test concern |

## 8. Open Questions - Final State of the Investigation Log

- `[RESOLVED] OQ-1 - Is the fallback call live for standard YR CMIN? -> Yes, stock CMIN/refinery flags reach `Mission_Harvest` state 2 fallback when close reservation/radio does not fire.` (evidence: `ini/rulesmd.ini:7361,7364,7396`; `0x0073E5E0`)
- `[RESOLVED] OQ-2 - What is the helper origin? -> Packed cell `refinery_anchor + QueueingCell`; stock GAREFN/NAREFN seed offset is `(4,1)`.` (evidence: `0x0073ED25`, `0x0073ED34`; `ini/artmd.ini:1716,1773`)
- `[RESOLVED] OQ-3 - Is pushed literal `2` a radius? -> No, it is the speed-type/passability argument. Radius comes from receiver `+0xF4 + +0xF8`, capped at 32.` (evidence: `0x0073ED5E`; `0x0056DCE1..0x0056DCF9`)
- `[RESOLVED] OQ-4 - What receiver supplies radius fields in this call? -> ECX is `0x0087F7E8`, the MapClass singleton; `+0xF4/+0xF8` are map Size width/height per MapClass docs.` (evidence: `0x0073ED66`; `MAPCLASS_GHIDRA_REPORT.md`)
- `[RESOLVED] OQ-5 - What are the exact CMIN fallback args? -> output, origin, speed=2, zone=-1, movement=0, bridge_aware=0, width=1, height=1, overlay=0, height_check=0, object_check=0, bridge_allowed=1, target={0,0}, skip=0, occupancy_rect=0.` (evidence: `0x0073ED42..0x0073ED75`)
- `[RESOLVED] OQ-6 - What coordinate order is scanned? -> Ring `r`: top row west->east, bottom row west->east, left column north->south excluding corners, right column north->south excluding corners.` (evidence: `0x0056DD14..0x0056E578`)
- `[RESOLVED] OQ-7 - Does ring 0 have a special duplicate? -> Yes, with `skip_first_quadrant=0`, origin is tested twice.` (evidence: first row-pair loop at `r=0` in `0x0056DD14..0x0056E141`)
- `[RESOLVED] OQ-8 - Does the helper return first passable cell? -> No. It collects up to 24, classifies direct/indirect, then modulo-selects for null target.` (evidence: `0x0056E5B3..0x0056E79A`)
- `[RESOLVED] OQ-9 - Does CMIN fallback call `CheckOccupancy`? -> No, final arg is zero, so the branch is disabled.` (evidence: `0x0073ED42`; `0x0056DE98`-style branches in helper)
- `[RESOLVED] OQ-10 - Are bridge cells rejected in this fallback? -> No; the bridge flag arg is nonzero, and helper only rejects bridge cells when that arg is zero.` (evidence: `0x0073ED4E`; helper `param_13 != 0 || !(flags & 0x100)`)
- `[RESOLVED] OQ-11 - Does the helper use target-distance selection here? -> No; target is `{0,0}`, so it uses `g_CurrentFrameCounter % count`.` (evidence: zeroed local target at `0x0073ED6B/70`; selection block)
- `[RESOLVED] OQ-12 - Are TS-only gates involved? -> No TS/Fog-only gate controls the scoped call. Branch liveness is content/state conditional, not TS-only.` (evidence: `0x0073E5E0`; stock YR INI)
- `[RESOLVED] OQ-13 - What test would pin this? -> `test_chrono_far_return_fallback_search_order_matches_gamemd` should block seed and selected ring candidates, run at controlled frame ticks, and assert modulo choice among first direct ring candidates.` (evidence: implementation handoff below)
- `[DEFERRED] OQ-14 - Full semantics of `FUN_006D6410` height-corrected lookup.` (category: out-of-scope; reason: prior helper report covers it sufficiently for direct/indirect distinction; this slice only needed call/selection behavior; next-step-if-pursued: dedicated visual-height projection audit)

## 9. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| CMIN fallback passes `speed_type=2`, `zone=-1`, `width=1`, `height=1`, `target={0,0}`, `skip=0`, `check_occupancy_rect=0`; literal `2` is not radius. | `0x0073ED42..0x0073ED75` | partial/unchecked: Rust uses path-grid passability and no argument object matching binary helper | `src/sim/miner/miner_system.rs:1052`, `src/sim/miner/miner_dock_sequence.rs:268`, future nearby-passable contract | Keep CMIN fallback as a 1x1 passability search from `QueueingCell`, with no extra occupancy-rect rejection and no caller radius=2. | `test_chrono_far_return_fallback_search_order_matches_gamemd`: blocked QueueingCell forces ring scan from `(rx+4,ry+1)` with same accepted candidates as gamemd. | Do not treat the pushed `2` as radius or 2x2 footprint. |
| Effective ring limit is `min(MapClass.Size.width + MapClass.Size.height, 32)`, so standard maps search rings `0..31`. | `0x0073ED66`; `0x0056DCE1..0x0056DCF9`; MapClass docs | mismatch: `EXIT_SEARCH_MAX_RADIUS=16` | `src/sim/miner/miner_dock_sequence.rs:39`, CMIN staging caller | Raise/parameterize this fallback's radius to binary-equivalent 32 for normal maps or derive from parsed map Size and cap. | Fully block rings 0..16 but leave a valid direct cell on ring 17; gamemd picks it, Rust should too. | Do not keep a 16-ring cap for CMIN far-return fallback. |
| Selection is not first-passable: collect up to 24 candidates, prefer direct candidates, modulo by `g_CurrentFrameCounter` when target is null. | `0x0056E5B3..0x0056E79A` | partial: current helper modulo-picks first non-empty ring but lacks direct/indirect split and 24 cap | `src/sim/miner/miner_dock_sequence.rs:268` | Preserve top/bottom/left/right order, 24 cap, direct-vs-indirect preference, and frame modulo for ties. | At a controlled tick with three passable ring-1 direct cells, selected cell is candidate `frame % 3`; indirect cells are ignored when any direct exists. | Do not return the first passable candidate for this fallback. |

## Negative Facts / Do Not Do

- Do not implement the Chrono Miner fallback `Find_Nearby_Passable_Cell` call with radius `2`; the pushed `2` is the passability speed type. Active in YR: Yes; evidence `0x0073ED5E`, helper parameter use.
- Do not use a 2x2 or refinery foundation-sized footprint here; the helper receives `width=1`, `height=1`. Active in YR: Yes; evidence `0x0073ED54..0x0073ED55`.
- Do not describe the cell-coordinate scan as a diamond if that implies Manhattan-distance perimeter. The binary scans square/Chebyshev perimeter rows and columns in cell coordinates. Active in YR: Yes; evidence `0x0056DD14..0x0056E578`.
- Do not add `CellRect__CheckOccupancy` or object-list safety filtering to this fallback unless another verified caller requires it; CMIN passes the check flag as `0`. Active in YR: Yes; evidence `0x0073ED42`.
- Do not pick first passable cell when several candidates exist; null target means frame-modulo selection from direct candidates if any. Active in YR: Yes; evidence `0x0056E5B3..0x0056E79A`.

## Remaining Uncertainty

- The report did not re-open `FUN_006D6410`; it relies on prior helper research for the direct/indirect visual-height classification. This does not affect call arguments or ring order.
- Exact runtime values of `MapClass+0xF4/+0xF8` are map-dependent; prior MapClass docs identify them as `[Map] Size` width/height, so ordinary playable maps exceed the 32 cap.
- `g_CurrentFrameCounter` vs Rust `sim.tick` equivalence was not proven in this slice.

## Stale Docs / Follow-up Docs

- `C:/Users/enok/Documents/ra2-rust-game-docs/FIND_NEARBY_PASSABLE_CELL_GHIDRA_REPORT.md`: replace "`this` (implicit) = FootClass*; search radius is derived from `this->Speed + this->SightRange`" with "`the helper reads receiver `+0xF4 + +0xF8`, capped at 32; caller identity matters. In the Chrono Miner far-return fallback, ECX is the MapClass singleton `0x0087F7E8`, so the effective radius comes from MapClass Size width+height, not the miner's speed/sight.`"
- `C:/Users/enok/Documents/ra2-rust-game-docs/FIND_NEARBY_PASSABLE_CELL_GHIDRA_REPORT.md`: replace "expanding diamond/ring pattern" with "expanding square/Chebyshev perimeter in cell coordinates: top row west-to-east, bottom row west-to-east, left column north-to-south excluding corners, right column north-to-south excluding corners."
- `C:/Users/enok/Documents/ra2-rust-game-docs/miner/CHRONO_MINER_FAR_RETURN_FALLBACK_DESTINATION_GHIDRA_REPORT.md`: replace "`Find_Nearby_Passable_Cell(seed, size=2, zone=-1, flags...)`" with "`Find_Nearby_Passable_Cell(seed, speed_type=2, zone=-1, width=1, height=1, target={0,0}, skip_first=0, check_occupancy_rect=0)`."
- Any in-repo fidelity note saying "radius=2 around QueueingCell" should become "QueueingCell seed; helper searches up to the helper radius cap, normally 32 rings on standard maps; literal `2` is SpeedType."

## Sources

- Ghidra read-only decompile: `UnitClass__Mission_Harvest @ 0x0073E5E0`.
- Ghidra read-only assembly context: `CALL 0x0056DC20` at `0x0073ED75`.
- Ghidra read-only decompile/assembly: `FootClass__Find_Nearby_Passable_Cell @ 0x0056DC20`.
- Ghidra read-only decompile: `CellRect__CheckPassability @ 0x0056E7C0`, `CellRect__CheckOccupancy @ 0x00586780`, `MapClass__Get_CellClass @ 0x005657A0`, `MapClass__constructor @ 0x00565090`.
- Starting docs: `miner/CHRONO_MINER_MISSION_HARVEST_STATE2_RETURN_BRANCH_COORDS_GHIDRA_REPORT.md`, `miner/CHRONO_MINER_FAR_RETURN_FALLBACK_DESTINATION_GHIDRA_REPORT.md`.
- Prior docs referenced: `FIND_NEARBY_PASSABLE_CELL_GHIDRA_REPORT.md`, `CELLRECT_PASSABILITY_OCCUPANCY_VALIDATORS_GHIDRA_REPORT.md`, `MAPCLASS_GHIDRA_REPORT.md`, `LOCAL_GRID_CELL_SKEW_TRANSFORMS_GHIDRA_REPORT.md`.
- INI checked: `ini/rulesmd.ini`, `ini/artmd.ini`.
- Rust scan only: `src/sim/miner/miner_system.rs`, `src/sim/miner/miner_dock_sequence.rs`, `src/sim/pathfinding/core.rs`, `src/rules/art_data.rs`.
