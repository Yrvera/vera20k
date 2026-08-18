# Bridge Two-Pass Can_Enter_Cell Split - Ghidra Research Report

**Addresses:** `0x0073F0A0` UnitClass::Can_Enter_Cell, `0x0051BF90` InfantryClass::Can_Enter_Cell, `0x004D9C60` CheckBridgeTraversal, `0x00429A90` AStar_main_loop, `0x0042A460` AStar_create_node, `0x004B0F20` DriveLocomotionClass::Process_Drive_Track, `0x0075AEC0` WalkLocomotionClass::ProcessMovement

**Confidence:** High for the Unit/Infantry/A*/CheckBridgeTraversal split; Medium for runtime locomotor call-site implications where decompiler recovery is less clean.

**Active in YR:** Yes. A* dispatches through vtable `+0x1AC` in standard pathfinding. Unit and Infantry vtables point `+0x1AC` to the functions above, and `+0x1B0` to `CheckBridgeTraversal`.

## 1. Overview

`Can_Enter_Cell` has two separate bridge/ground decisions:

1. A pre-`CheckBridgeTraversal` byte chooses which object list is scanned (`Cell+0xE4` ground list or `Cell+0xE8` bridge/alternate list).
2. A later post-`CheckBridgeTraversal` test chooses which occupancy flag cache is used (`Cell+0x124/+0x54` ground occupancy or `Cell+0x128/+0x58` bridge occupancy).

Those decisions are not always the same. The binary can scan the bridge object list while using the ground occupancy bits. This is live for normal YR pathfinding, but the normal A* caller passes the current path height, not a permanent unknown-height sentinel, so the live A* split is narrower than older shorthand notes suggested.

## 2. Key Offsets

| Offset | Owner | Meaning in this investigation | Evidence |
|---:|---|---|---|
| `+0x11B` | `CellClass` | Signed cell level byte used in all height comparisons | `UnitClass::Can_Enter_Cell` asm `MOVSX ... [ECX+0x11B]`; `CheckBridgeTraversal` asm |
| `+0x11C` | `CellClass` | Slope byte required for one-level transitions | `CheckBridgeTraversal` asm tests `[cell+0x11C]` in abs-diff-1 branch |
| `+0x124` | `CellClass` | Ground occupancy flags dword/byte source | `UnitClass::Can_Enter_Cell` asm `0x0073F0ED-0x0073F109`; Infantry asm `0x0051BFD2-0x0051BFEE` |
| `+0x128` | `CellClass` | Bridge/alternate occupancy flags dword/byte source | Unit asm `0x0073F32C-0x0073F348`; Infantry asm `0x0051C11A-0x0051C136` |
| `+0x54` | `CellClass` | Ground occupancy owner/blocker field copied with ground flags | Unit asm `MOV EDX,[ECX+0x54]` at `0x0073F0F3` |
| `+0x58` | `CellClass` | Bridge occupancy owner/blocker field copied with bridge flags | Unit asm `MOV EDX,[EDX+0x58]` at `0x0073F336` |
| `+0xE4` | `CellClass` | Ground object content list head | Unit asm `MOV ESI,[EDI+0xE4]` at `0x0073F51A`; Infantry asm `0x0051C23F` |
| `+0xE8` | `CellClass` | Bridge/alternate object content list head | Unit asm `MOV ESI,[EDI+0xE8]` at `0x0073F506`; Infantry asm `0x0051C233` |
| `+0x140 bit 0x100` | `CellClass` | Bridge cell flag | Tested throughout Unit/Infantry/A*/CBT |
| `+0x140 bit 0x200` | `CellClass` | Bridgehead/transition flag for high-bridge entry | `CheckBridgeTraversal` asm tests `0x200` before allowing/forcing bridge pass |
| `+0x30` | object list node | Next pointer used to walk selected object list | Unit asm advances `ESI=[ESI+0x30]` at `0x0073FA87`; Infantry asm `0x0051C70F` |
| `+0x8C` | `ObjectClass` | Persistent `OnBridge` byte, used by pathfinding start state and locomotor bridge checks | `AStar_main_loop` asm reads `[object+0x8C]` at `0x00429B32` |
| `+0x30` | `PathfinderClass` | Current expanded path height | A* asm uses `[ESI+0x30]` in bridge-layer test and passes it to vtable `+0x1AC` |
| `+0x18/+0x24` | `PathfinderClass` | Ground closed/cost arrays | A* asm writes these when the candidate is classified ground |
| `+0x1C/+0x20` | `PathfinderClass` | Bridge closed/cost arrays | A* asm writes these when the candidate is classified bridge |

## 3. Verified Binary Logic

### 3.1 UnitClass pre-pass list selection

`UnitClass::Can_Enter_Cell` sets a stack byte at `[ESP+0x13]` before calling `CheckBridgeTraversal`.

Pseudocode from asm `0x0073F0B7-0x0073F0ED`:

```text
if (!(cell.flags & 0x100)) {
    list_bridge = false;
} else if (height != -1 && abs(height - cell.level) <= 1) {
    list_bridge = false;
} else {
    list_bridge = true;
}
```

Tiny details:

- The comparison is `CMP EAX,0x1` followed by `JLE`, so the ground case is `abs(...) <= 1`, equivalent to `< 2`.
- `height == -1` bypasses the abs check and selects the bridge list for bridge cells.
- This byte is passed by address to vtable `+0x1B0` (`CheckBridgeTraversal`) at `0x0073F2D2-0x0073F2EB`.
- The byte is not recomputed after `CheckBridgeTraversal`; later code directly tests `[ESP+0x13]`.

### 3.2 UnitClass pre-pass occupancy cache

Immediately after the list byte is set, UnitClass copies ground occupancy state:

```text
stack_occ_flags = cell+0x124 low byte
stack_occ_moving_bit = (cell+0x124 >> 5) & 1
stack_occ_owner = cell+0x54
```

Evidence:

- `0x0073F0ED`: `MOV AL, byte ptr [ECX+0x124]`
- `0x0073F0F3`: `MOV EDX, dword ptr [ECX+0x54]`
- `0x0073F100-0x0073F109`: shifts `Cell+0x124` by 5, masks bit 0, and stores it separately.

This happens before `CheckBridgeTraversal`.

### 3.3 UnitClass post-pass occupancy overwrite

After `CheckBridgeTraversal` returns non-7, UnitClass may overwrite the occupancy cache from the bridge fields:

```text
if (height != -1 &&
    (cell.flags & 0x100) &&
    height == cell.level + 4) {
    stack_occ_flags = cell+0x128 low byte
    stack_occ_moving_bit = (cell+0x128 >> 5) & 1
    stack_occ_owner = cell+0x58
}
```

Evidence:

- `0x0073F303-0x0073F32A`: tests height != -1, bridge flag, and `height == cell.level + 4`.
- `0x0073F32C-0x0073F348`: copies `Cell+0x128`, `(Cell+0x128 >> 5) & 1`, and `Cell+0x58` into the same stack cache used later.

Crucial ordering: this occupancy overwrite occurs after `CheckBridgeTraversal`, but before shroud, locomotor passability, overlay checks, object-list scan, and final occupancy-bit classification.

### 3.4 UnitClass list scan uses the pre/CBT byte, not the post occupancy predicate

The object list is selected at `0x0073F4F9-0x0073F520`:

```text
if (list_bridge) {
    obj = cell.bridge_list_head;  // cell+0xE8
} else {
    obj = cell.ground_list_head;  // cell+0xE4
}
```

The scan follows only the selected chain:

```text
while (obj != null) {
    ...
    obj = obj.next; // +0x30
}
```

Evidence:

- `0x0073F4F9`: reads `[ESP+0x13]`.
- `0x0073F506`: if nonzero, loads `[EDI+0xE8]`.
- `0x0073F51A`: if zero, loads `[EDI+0xE4]`.
- `0x0073FA87`: advances through `[ESI+0x30]`.

Therefore, same-cell bridge/ground asymmetry is real: the function can select one content list while using occupancy bits from another layer.

### 3.5 InfantryClass uses the same split, with one extra early return

`InfantryClass::Can_Enter_Cell` repeats the same structure:

- Pre-list byte at `[ESP+0x11]`.
- Ground occupancy cache from `Cell+0x124` and `Cell+0x54`.
- Calls vtable `+0x1B0` with pointer to the list byte.
- Post-CBT bridge occupancy overwrite from `Cell+0x128` and `Cell+0x58`.
- Selects `Cell+0xE8` when list byte is true, else `Cell+0xE4`.

Evidence:

- Pre list byte: `0x0051BFA2-0x0051BFD2`.
- Ground occupancy cache: `0x0051BFD2-0x0051BFEE`.
- Infantry-only early return: `0x0051C0B2-0x0051C0CD`.
- CBT call: `0x0051C0D0-0x0051C0EC`.
- Bridge occupancy overwrite: `0x0051C0FB-0x0051C13A`.
- List selection: `0x0051C225-0x0051C249`.

Infantry-only tiny detail:

```text
if (height - cell.level > 4) return 0;
```

This happens before `CheckBridgeTraversal`, object-list selection, and post-bridge occupancy overwrite. It is not a bridge split, but it is an active passability shortcut and should not be lost when comparing infantry pathing.

## 4. CheckBridgeTraversal Details

`CheckBridgeTraversal` is the virtual at `+0x1B0`. Unit and Infantry both call it from their `Can_Enter_Cell`.

### 4.1 Argument roles

For the `Can_Enter_Cell` callers in this report:

```text
candidate cell C = stack arg 1
direction        = stack arg 2
height in/out    = stack arg 3, pointer
list byte in/out = stack arg 4, pointer
parent cell P    = stack arg 5, optional
```

If `P` is null, the helper computes an adjacent cell using `(direction - 4) & 7` and the global direction-offset table. Evidence: `0x004D9C70-0x004D9CBA`.

### 4.2 Height seeding

If `direction == -1` and `*height == -1` and the candidate is a bridge cell, it writes:

```text
*height = C.level + 4
```

Evidence: `0x004D9E3E-0x004D9E5C`.

If `direction != -1`, `P` exists, and `*height == -1`, then if `P` is bridge:

```text
*height = P.level + 4
if (!(C.flags & 0x200)) return 7
```

Evidence: `0x004D9CD5-0x004D9D0E`.

Tiny detail: this gate checks candidate `0x200` after seeding from the parent deck. It is not enough for the parent to be a bridge cell.

### 4.3 Difference cases

The helper computes a signed level difference. If `P` is a bridge cell, it compares `P.level` to `C.level`; otherwise it compares `*height` to `C.level`.

Verified branches:

- `abs(diff) == 0`: allowed, except certain bridge/current-height contradictions return 7. If `*height != -1` and `*height != C.level`, the fallback returns 7.
- `abs(diff) == 1`: requires a nonzero slope byte. If `diff > 0`, it tests `C+0x11C`; otherwise it tests `P+0x11C`.
- `abs(diff) == 4`: all other height jumps return 7.
- For one `abs(diff) == 4` orientation, `P.level == C.level - 4` requires `*height == C.level` and `P` bridge.
- For the opposite orientation, `C.level == P.level - 4` requires `C` bridge and `C` bridgehead, then writes `*list_byte = 1` and returns 0.

Evidence:

- Abs-diff dispatch: `0x004D9D32-0x004D9D50`.
- Diff-1 slope test: `0x004D9DD8-0x004D9E05`.
- Diff-4 branch requiring parent bridge/height match: `0x004D9D5C-0x004D9D8F`.
- Diff-4 branch requiring candidate bridge+bridgehead and forcing list byte: `0x004D9D8F-0x004D9DD5`.

Important: `CheckBridgeTraversal` only writes the list byte to `1`; this investigation found no path where it clears a previously bridge-selected byte.

## 5. Normal A* Pathfinding Activity

### 5.1 A* passes a concrete current path height

`AStar_main_loop` dispatches to vtable `+0x1AC` at `0x00429F54`:

```text
candidate_cell = EBX
direction      = neighbor direction
height         = Pathfinder+0x30
parent_cell    = current node's cell
flags          = Pathfinder+0x08 low byte
```

Evidence:

- Current path height loaded from `[ESI+0x30]` immediately before the call: `0x00429F45-0x00429F4F`.
- Candidate cell pushed from `EBX`: `0x00429F51`.
- Parent/current cell pointer pushed from the current node: `0x00429F43-0x00429F4B`.
- Virtual call `CALL [EDX+0x1AC]`: `0x00429F54`.

This corrects an easy overstatement: normal A* expansions are not generally calling `Can_Enter_Cell` with `height == -1`. They pass the current node's path height.

### 5.2 A* has its own pre-candidate bridge/ground closed-list split

At `0x00429E54-0x00429E7F`, A* classifies the candidate cell against `Pathfinder+0x30`:

```text
if ((candidate.flags & 0x100) &&
    abs(path_height - candidate.level) > 1) {
    candidate_is_ground_for_astar = false;
} else {
    candidate_is_ground_for_astar = true;
}
```

The stack byte polarity is inverted relative to the Unit/Infantry list byte:

- A* `[ESP+0x60] == 1` means ground.
- A* `[ESP+0x60] == 0` means bridge.

Evidence:

- Sets `[ESP+0x60] = 0` before the abs check when the candidate has bridge flag.
- If abs is not greater than 1, falls to `[ESP+0x60] = 1`.
- Ground arrays `Pathfinder+0x18/+0x24` are used when `[ESP+0x60] != 0`.
- Bridge arrays `Pathfinder+0x1C/+0x20` are used when `[ESP+0x60] == 0`.

### 5.3 AStar_create_node updates the next node height after Can_Enter_Cell

`AStar_create_node` stores the next node height after a candidate has been accepted.

Relevant verified behavior at `0x0042A460`:

```text
next_height = candidate.level

if (candidate.flags & 0x100) {
    if (parent.flags & 0x100 &&
        parent_node.height == parent.level + 4) {
        next_height = candidate.level + 4
    } else if (!(parent.flags & 0x100) &&
               abs(candidate.level - parent_node.height + 3) <= 1) {
        next_height = candidate.level + 4
    }
}
```

The second case is effectively a broad internal height-carry condition (`parent_height - candidate.level` in the 2..4 range), but `CheckBridgeTraversal` still filters illegal 2/3 height jumps before the node is created.

## 6. Runtime Locomotor Activity

### 6.1 Drive locomotor

`DriveLocomotionClass::Process_Drive_Track` calls vtable `+0x1AC` during track chaining and collision handling.

Evidence from decompile at `0x004B0F20`:

```text
height = CellClass::Get_Effective_Height(...)
candidate = MapClass::Get_CellClass(...)
result = object->vtable[0x1AC](candidate, direction, height, ...)
```

This is active in normal YR drive locomotion. The decompiler recovered fewer explicit stack arguments than the callee signature, so the exact parent-cell argument for this specific call site is lower confidence than A*.

### 6.2 Walk locomotor

`WalkLocomotionClass::ProcessMovement` also calls vtable `+0x1AC` during infantry movement:

```text
CellClass::Get_Effective_Height(...)
candidate = MapClass::Get_CellClass(...)
result = object->vtable[0x1AC](...)
```

Evidence: decompile around `0x0075B5xx-0x0075B6xx`.

Walk locomotion also has bridge/ground `OnBridge` transition writes later in the same function (`Object+0x8C`), but those are separate from the `Can_Enter_Cell` two-pass list/occupancy split.

## 7. Derived Split Cases

### 7.1 Binary-verified invariant

The binary has independent state for:

```text
object_list_layer = pre/CBT list byte
occupancy_bits_layer = post-CBT height == cell.level + 4 predicate
```

These are read separately:

- The object list layer selects `Cell+0xE4` vs `Cell+0xE8`.
- The occupancy bits layer selects `Cell+0x124/+0x54` vs `Cell+0x128/+0x58`.

### 7.2 High-confidence inference: bridge-list/ground-bits is the real divergence direction

I found paths where the list byte can be bridge while the post occupancy predicate remains ground:

1. Pre-pass selects bridge because the candidate is a bridge cell and `abs(height - candidate.level) > 1`, but final `height != candidate.level + 4`.
2. `CheckBridgeTraversal` force-sets the list byte to bridge in the candidate bridgehead diff-4 branch, but it does not guarantee the post occupancy predicate is true.

I did not find a path where the post occupancy predicate selects bridge while the list byte remains ground:

- Post bridge occupancy requires candidate bridge and `height == candidate.level + 4`.
- If the incoming height already equals `candidate.level + 4`, the pre-pass selects bridge because abs is 4.
- If `height == -1`, pre-pass selects bridge for bridge candidates before `CheckBridgeTraversal` can seed a deck height.
- `CheckBridgeTraversal` can set the list byte to 1, but this investigation found no path that clears it.

Confidence: High for the observed write behavior; Medium for the "no ground-list/bridge-bits path" conclusion because it depends on all relevant writes being visible in these functions.

### 7.3 Normal A* consequence

Because A* passes `Pathfinder+0x30`, not `-1`, old examples that rely on a generic `height == -1` first pass should not be treated as normal A* cases.

The live A* split to test is:

```text
current path height causes A*/Can_Enter_Cell list classification as bridge,
but CheckBridgeTraversal leaves or rewrites height to a value that is not candidate.level + 4,
so occupancy flags remain ground.
```

Player-visible condition:

- A bridge and ground occupant must be asymmetric in the same cell.
- The candidate must be on/near a bridge transition or height edge where the two predicates diverge.
- The scanned object list and occupancy bits must drive different return codes or costs.

This is uncommon in open terrain but relevant because bridge cells are chokepoints and frequently contain queued units.

## 8. Current Rust Implementation Status

No Rust code was changed in this investigation.

### 8.1 A* uses one bridge decision for multiple concerns

Current Rust A* computes one `neighbor_use_bridge` in `src/sim/pathfinding/core.rs`:

- `core.rs:116`: `is_at_bridge_level(path_height, cell)`
- `core.rs:425`: `let neighbor_use_bridge = is_at_bridge_level(current.height, neighbor_cell);`
- `core.rs:428`: `let neighbor_height = compute_neighbor_height(...)`
- `core.rs:450`, `core.rs:503`, `core.rs:524`, `core.rs:606`, `core.rs:621`: the same `neighbor_use_bridge` feeds closed-list, entity-block, cost, came-from, and output layer decisions.

This is close to the binary A* pre-classification for the closed-list side, but it cannot model the later `Can_Enter_Cell` occupancy overwrite.

### 8.2 Cell entry classification uses one target layer

`src/sim/pathfinding/cell_entry.rs` currently takes a single `target_layer`:

- `cell_entry.rs:91`: `check_terrain(... target_layer ...)`
- `cell_entry.rs:120`: infantry subcell allocation uses `target_layer`.
- `cell_entry.rs:131`: vehicle occupancy uses `is_empty_on(target_layer)`.
- `cell_entry.rs:152`: `classify_occupied_cell(... target_layer ...)`.
- `cell_entry.rs:167-179`: crush-victim checks use `target_layer`.
- `cell_entry.rs:189-191`: primary blocker lookup uses `target_layer`.
- `cell_entry.rs:228`: selected occupants come from `occ.iter_layer(layer)`.

The module header explicitly calls this a known parity boundary and says it approximates the post-switch output.

### 8.3 Runtime movement/deferred occupancy also uses one target layer

`src/sim/movement/movement_occupancy.rs` also uses a single `next_layer`:

- `movement_occupancy.rs:38-73`: deferred check tests blockers/subcells on `next_layer`.
- `movement_occupancy.rs:175-177`: deferred blocker classification passes `next_layer` to `classify_occupied_cell`.
- `movement_occupancy.rs:249-250` and `354-355`: scatter uses `next_layer`.

This cannot express "scan bridge object list, then classify ground occupancy bits" as gamemd can.

## 9. Future Implementation Invariants

This section is research guidance only; no implementation is provided here.

A future parity implementation should preserve these behavioral invariants:

1. Keep two distinct layer decisions in cell-entry logic:
   - `list_layer`: pre-CBT candidate list byte, mutable by `CheckBridgeTraversal`.
   - `occupancy_bits_layer`: post-CBT predicate `height != -1 && candidate.bridge && height == candidate.level + 4`.
2. Do not recompute `list_layer` after the post-CBT occupancy overwrite.
3. Scan only the selected object list. Do not scan both layers to "be safe."
4. Occupancy bits/owner classification must be read from the post-CBT occupancy layer, even when the object list layer differs.
5. Preserve the exact threshold: bridge list pre-pass uses `abs(height - cell.level) > 1`, not `>= 1`.
6. Preserve signed cell level behavior: `Cell+0x11B` is read with `MOVSX`.
7. Preserve `CheckBridgeTraversal` ordering: it may mutate height before the post occupancy predicate runs, and may set the list byte to bridge before the object list scan.
8. For normal A*, pass the current node path height into the cell-entry check. Do not model normal A* as always passing `-1`.
9. For stale/duplicate occupancy edge cases, gamemd behavior is selected-list based. A Rust helper that removes or checks by id across all layers can erase stale-list states that gamemd would still expose or ignore depending on selected layer.

## 10. Duplicate/Stale-List Edge Cases

Binary-verified:

- Unit and Infantry scan exactly one list head (`+0xE4` or `+0xE8`) and advance through `Object+0x30`.
- They do not scan both lists if the selected list is empty.
- The occupancy bit cache can come from a different layer than the scanned object list.

Inference:

- If a stale duplicate exists only in the nonselected list, gamemd's selected-list scan will not see it in this function.
- If the same object is stale in both lists, gamemd can see it through whichever list is selected.
- Rust functions that remove an id across all layers are desirable for cleanup, but they are not equivalent to gamemd's selected-list read semantics for reproducing stale-list bugs.

This matters most for bridge-collapse/drop-in and same-cell relayer bugs, where transient stale entries are plausible if ordering is wrong.

## 11. Corrections To Prior Notes

This report refines `G6_TWO_PASS_DIVERGENCE_SUPPLEMENT.md`.

Confirmed:

- The two-pass split is real.
- vtable `+0x1AC` is the `Can_Enter_Cell` A* entry.
- vtable `+0x1B0` is `CheckBridgeTraversal`.
- Unit and Infantry both have the split.

Refined:

- Normal A* passes a concrete current path height from `Pathfinder+0x30`, not a generic `-1` height on every expansion.
- Therefore, generic `height == -1` bridge-list examples are valid for some callers, but should not be used as the primary normal-A* example unless a specific caller passing `-1` is identified.
- The practical A* fidelity probe should search for bridge-list/ground-bits cases driven by current path height and `CheckBridgeTraversal` height mutation/force-list behavior.

## 12. Open Questions

1. Which concrete retail maps produce a standard A* expansion where `list_layer == bridge` and `occupancy_bits_layer == ground` with asymmetric occupants in the same cell?
2. Which non-A* callers pass `height == -1` to Unit/Infantry `Can_Enter_Cell` in standard YR gameplay, if any?
3. Does a runtime drive/walk locomotor call site pass an explicit parent cell pointer to `CheckBridgeTraversal`, or does it rely on the helper's `(direction - 4) & 7` fallback? A* is verified; runtime locomotor decompilation is less clean.
4. Does any standard YR bridge-collapse/drop-in path create temporary stale duplicates across `+0xE4/+0xE8` that would make selected-list semantics player-visible?
5. Does AircraftClass or other unusual movers have a meaningful equivalent split, or are Unit/Infantry the only relevant ground locomotion cases for bridge chokepoints?

## Sources

- Ghidra live decompilation/disassembly of `gamemd.exe`:
  - `0x0073F0A0` UnitClass::Can_Enter_Cell
  - `0x0051BF90` InfantryClass::Can_Enter_Cell
  - `0x004D9C60` CheckBridgeTraversal
  - `0x00429A90` AStar_main_loop
  - `0x0042A460` AStar_create_node
  - `0x0042C900` AStar_pathfind_search
  - `0x0042ACF0` PathfinderClass::UpdateBridgePassability
  - `0x004B0F20` DriveLocomotionClass::Process_Drive_Track
  - `0x0075AEC0` WalkLocomotionClass::ProcessMovement
- Ghidra xrefs:
  - Unit `+0x1AC`: data xref `0x007F5E1C -> 0x0073F0A0`
  - Infantry `+0x1AC`: data xref `0x007EB204 -> 0x0051BF90`
  - `CheckBridgeTraversal`: data xrefs `0x007E2454`, `0x007E8E44`, `0x007EB208`, `0x007F5E20`
  - `AStar_main_loop`: caller `AStar_pathfind_search @ 0x0042C900`
- Existing reports:
  - `C:/Users/enok/Documents/ra2-rust-game-docs/G6_TWO_PASS_DIVERGENCE_SUPPLEMENT.md`
  - `C:/Users/enok/Documents/ra2-rust-game-docs/BRIDGE_CAN_ENTER_CELL_HIERARCHY_GHIDRA_REPORT.md`
  - `C:/Users/enok/Documents/ra2-rust-game-docs/BRIDGE_CHECK_TRAVERSAL_AND_CELL_OFFSETS_GHIDRA_REPORT.md`
  - `C:/Users/enok/Documents/ra2-rust-game-docs/BRIDGE_DEFERRED_MECHANICS_GHIDRA_REPORT.md`
- Rust status scan:
  - `C:/Users/enok/Documents/ra2-rust-game/src/sim/pathfinding/core.rs`
  - `C:/Users/enok/Documents/ra2-rust-game/src/sim/pathfinding/cell_entry.rs`
  - `C:/Users/enok/Documents/ra2-rust-game/src/sim/movement/movement_occupancy.rs`

