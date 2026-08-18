# Low Bridge TubeClass Producers and Lifecycle -- Ghidra Follow-up

Date: 2026-05-16

Scope: focused follow-up to `LOW_BRIDGE_TUBECLASS_GHIDRA_REPORT.md` and `LOW_BRIDGE_TUBECLASS_DOC_VERIFICATION.md`.

Addresses checked:

- `DriveLocomotionClass::Process_Drive_Track @ 0x004B0F20`
- `WalkLocomotionClass::ProcessMovement @ 0x0075AEC0`
- active tube-state write sites at `0x004B1380`, `0x0051515B`, `0x005B06E6`, `0x006A0A48`, `0x0075B3FC`
- active tube-state clear sites at `0x004D338F`, `0x0051BA8D`, `0x00735FF8`
- `FUN_00728280` tube save/compaction
- `FUN_007283C0` `[Tubes]` parser
- direct `CellClass+0x116` write sites found by byte-pattern audit

Confidence: High for Drive/Walk active tube-state producer behavior and direct `CellClass+0x116` writes found by searched instruction patterns. Medium for class-wide locomotor coverage because Hover/Mech/Ship producer sites were pattern/context verified but not fully line-by-line decompiled here. Low for retail map `[Tubes]` coverage because plain-text map search does not inspect MIX-packed maps.

Active in YR: Yes. The checked producers are in live locomotion processing functions. `UnitClass::TubeMovement` and infantry tube movement consume the state they produce.

## Summary

The remaining contradiction is resolved more sharply:

- `CellClass::RecalcAttributes` can create same-cell TubeClass shells with `path_len == 0`.
- `[Tubes]` map data can create fully initialized TubeClass records with entry, exit, path steps, and nonzero step count.
- Live locomotion enters tube movement when the current path direction is `8`. That producer reads the current cell's `CellClass+0x116`, stores the tube index into the object at `+0x684`, clears `+0x685`, and initializes a tube traversal destination from `TubeClass+0x28`.
- The checked Drive and Walk producer branches divide by `TubeClass+0x1C0` while setting the first in-tube Z/interpolation point. A zero-step same-cell shell is therefore not a valid practical input to direction-8 tube traversal.

Implementation implication: Rust needs both concepts. Same-cell shells exist and matter for predicates/zone/click logic, but direction-8 visible tube traversal should be fed by fully initialized tubes with valid step data, not by zero-step shells.

## Active Tube-State Producers

The active tube state consumed by unit/infantry tube movement is stored on the object:

- `object+0x684`: signed byte active tube index, `0xFF` means inactive.
- `object+0x685`: byte tube path cursor, reset to `0` on entry.
- `object+0x5E0..`: current path buffer.
- `object+0x5E4..`: following path buffer copied forward when tube traversal begins.
- `object+0x63C`: path/cache field reset to `-1` when traversal begins.

Direct write-site audit for active tube index:

| Address | Instruction | Function / role | Meaning |
|---:|---|---|---|
| `0x004B1380` | `MOV byte ptr [EAX + 0x684],DL` | `DriveLocomotionClass::Process_Drive_Track` | producer for wheeled/tracked drive locomotion |
| `0x0051515B` | `MOV byte ptr [EDX + 0x684],AL` | hover locomotion move path | producer with same surrounding tube setup pattern |
| `0x005B06E6` | `MOV byte ptr [ECX + 0x684],AL` | mech locomotion path | producer with same surrounding tube setup pattern |
| `0x006A0A48` | `MOV byte ptr [EAX + 0x684],DL` | `ShipLocomotionClass::Process_Drive_Track` | producer with same surrounding tube setup pattern |
| `0x0075B3FC` | `MOV byte ptr [ECX + 0x684],AL` | `WalkLocomotionClass::ProcessMovement` | infantry/walk producer |
| `0x004D338F` | `MOV byte ptr [ESI + 0x684],0xFF` | FootClass initialization | inactive initial state |
| `0x0051BA8D` | `MOV byte ptr [ESI + 0x684],0xFF` | infantry tube movement exit | clear after infantry exits tube |
| `0x00735FF8` | `MOV byte ptr [ESI + 0x684],0xFF` | `UnitClass::TubeMovement` exit | clear after unit exits tube |

The producer is not `GetTubeAtCell` by itself. It is a locomotion transition triggered by path direction `8`.

## Drive Locomotion Producer

In `DriveLocomotionClass::Process_Drive_Track @ 0x004B0F20`, the branch is entered when:

```text
object.path_current == 8
and drive_loco.current_track == -1
```

The verified behavior is:

1. Get the object's current cell through the object vtable.
2. Read `s16 tube_index = *(cell+0x116)`.
3. Require `0 <= tube_index < g_TubeCount`.
4. Load `tube = g_TubeArray[tube_index]`.
5. Set the locomotor destination to the tube exit coord `tube+0x28`, converted to world center `(x*256+128, y*256+128, z=0)`.
6. Copy `object+0x5E4..` to `object+0x5E0..` for `0x17` dwords.
7. Write `object+0x63C = -1`.
8. Write `object+0x684 = (byte)tube_index` and `object+0x685 = 0`.
9. Read the first tube path step `tube+0x30`, mask it with `& 7`, and compute the next in-tube center from `tube+0x24`.
10. Compute Z interpolation with:

```text
object+0x570 = (exit_ground_height - current_ground_height) / tube.path_len
             + current_ground_height
```

where `tube.path_len` is `*(int *)(tube+0x1C0)`.

There is no zero guard before the division by `tube+0x1C0` in this branch.

## Walk Locomotion Producer

`WalkLocomotionClass::ProcessMovement @ 0x0075AEC0` has the parallel path:

1. It checks `object+0x5E0 == 8`.
2. It reads the current cell's `+0x116`.
3. It requires the tube index to be within `g_TubeCount`.
4. It loads `g_TubeArray[index]`.
5. It sets the walk locomotor target to `TubeClass+0x28`.
6. It copies `object+0x5E4..` to `object+0x5E0..`.
7. It writes `object+0x63C = -1`, `object+0x684 = index`, and `object+0x685 = 0`.
8. It uses `TubeClass+0x24`, `TubeClass+0x30`, and `TubeClass+0x1C0` to set the first in-tube center and Z interpolation.

This branch also divides by `tube+0x1C0` without a zero guard.

If no valid tube exists, it clears the current path direction to `-1`, clears the locomotor destination, and calls the object's stop/repath handling.

## Consumer / Exit Behavior

The existing report's `UnitClass::TubeMovement @ 0x007359F0` finding is confirmed by the producer trace:

- `UnitClass::AI @ 0x007363B0` calls `UnitClass::TubeMovement` when signed byte `Unit+0x684` is non-negative.
- `UnitClass::TubeMovement` reads `g_TubeArray[Unit+0x684]`, path cursor `+0x685`, tube entry `+0x24`, exit `+0x28`, direction `+0x2C`, steps `+0x30`, and length `+0x1C0`.
- On exit it places the unit at `TubeClass+0x28`, writes `Unit+0x684 = 0xFF`, sets movement flags, and updates facing/state.

Infantry has a parallel tube movement routine. Its exit path clears `+0x684` at `0x0051BA8D` and sets `+0x68B = 1`.

`UnitClass::Receive_Radio` also observes active tube state: in message case `0x24`, it returns a different response when `(char)(Unit+0x684) != -1`. This is a small but player-visible integration point because tube-active units can answer object interaction queries differently while inside traversal.

## `CellClass+0x116` Direct Write Audit

Direct write pattern searches found these semantic writers:

| Address | Instruction class | Function / role | Meaning |
|---:|---|---|---|
| `0x0047BC48` | `MOV word ptr [ESI+0x116],AX` | `CellClass` constructor/init | initializes cell tube index, with `AX == 0xFFFF` in surrounding init |
| `0x00565F30` | copy from source cell | cell/map copy or serialization helper | copies `+0x116` as part of a cell state block |
| `0x00566742` | copy from source cell | cell/map copy or serialization helper | copies `+0x116` as part of a cell state block |
| `0x007280B7` | `MOV word ptr [EAX+0x116],CX` | `TubeClass::Constructor` | writes the new tube index to the entry cell unless coord is `(0,0)` |
| `0x0072824A` | `MOV word ptr [...+0x116],0xFFFF` | tube save/compaction path | clears entry cell when tube no longer qualifies as low bridge |
| `0x0072825F` | `MOV word ptr [...+0x116],AX` | tube save/compaction path | writes renumbered tube index |
| `0x007282E1` | `MOV word ptr [...+0x116],0xFFFF` | `FUN_00728280` | clears entry cell when tube no longer qualifies as low bridge |
| `0x007282F6` | `MOV word ptr [...+0x116],AX` | `FUN_00728280` | writes renumbered tube index |
| `0x00728519` | `MOV word ptr [...+0x116],CX` | `[Tubes]` parser `FUN_007283C0` | writes parsed tube index to parsed entry cell |
| `0x00728776` | `MOV word ptr [...+0x116],0xFFFF` | TubeClass removal/destructor-side cleanup | clears entry cell if it still points at the removed tube |

Search patterns used:

- `66 89 ?? 16 01 00 00` for register word writes to `[reg+0x116]`
- `66 C7 ?? 16 01 00 00 FF FF` for immediate clear writes to `[reg+0x116]`

No direct write found in the low bridge damage/repair helper family in this pattern audit. This does not prove an impossible absence of every exotic computed-offset write, but it materially strengthens the prior conclusion: normal low bridge damage/repair should not be modeled as directly deleting tube records unless another verified path proves it.

## Tube Save / Compaction

`FUN_00728280` writes the `[Tubes]` section and first compacts live tubes:

```text
for each tube:
    entry_cell = MapClass::Get_CellClass(tube.entry)
    if !entry_cell.IsLowBridgeCell():
        delete tube through vtable+0x20
        decrement local tube count
        entry_cell.tube_index = -1
        retry current index
    else:
        next_index += 1
        entry_cell.tube_index = next_index
```

After that pass it serializes each remaining tube as:

```text
entry_x, entry_y, direction, exit_x, exit_y, path[0..99]
```

Important nuance: this cleanup is tied to tube save/compaction, not to the live low bridge damage handler itself in the checked evidence.

## `[Tubes]` Parser

`FUN_007283C0` reads each `[Tubes]` entry and constructs a TubeClass with a `(0,0)` placeholder, then overwrites:

- `tube+0x24/+0x26`: entry X/Y
- `tube+0x2C`: direction
- `tube+0x28/+0x2A`: exit X/Y
- `tube+0x30...`: up to 100 path step entries
- `tube+0x1C0`: parsed step count

It then writes:

```text
MapClass::Get_CellClass(tube.entry)->tube_index = parsed_entry_index
```

This is the verified producer of fully initialized entry/exit/step tubes from map data.

## Zero-Step Shell vs Direction-8 Traversal

The binary supports both statements, but in different contexts:

- Same-cell zero-step tubes are real: `TubeClass::Constructor` and the `RecalcAttributes` low/tunnel branch create them.
- Visible direction-8 tube traversal requires a usable `TubeClass+0x1C0` step count in checked Drive/Walk producer code because the producer divides by it before handing control to tube movement.

Therefore the safest implementation rule is:

```text
Do not emit/consume a direction-8 locomotion transition for a tube whose path_len is 0.
```

Same-cell shells may still be live and important for predicates, cursor/action routing, zone record discovery, and save/compaction decisions.

## Retail Map `[Tubes]` Check

Plain-text search command:

```text
rg "\[Tubes\]" "C:/Users/enok/Documents/Command and Conquer Red Alert II" "C:/Users/enok/Documents/ra2-rust-game" -n --glob "*.map" --glob "*.yrm" --glob "*.mpr" --glob "*.ini"
```

Result: no hits in plain-text files visible at those paths.

Confidence: Low. Retail maps can be MIX-packed, compressed, or otherwise not visible as plain text to this search. This check should not be treated as evidence that retail YR never uses `[Tubes]`.

## Rust Implications

Do not implement Rust code from this report yet, but a later implementation plan should account for:

1. `CellClass+0x116` equivalent as a signed tube index separate from overlay identity.
2. `TubeClass` data with entry, exit, direction, step buffer, and step count.
3. Two creation modes:
   - auto same-cell shells from final `LandType == Tunnel(10)` plus tile-range predicates;
   - explicit parsed tubes from `[Tubes]` data.
4. A direction-8 path step that triggers a locomotion-side tube-entry transition.
5. Active object tube state equivalent to `+0x684/+0x685`.
6. A guard or path-planner invariant that prevents direction-8 traversal through zero-step shell tubes.
7. Low bridge damage/repair should update overlay/state/zones without assuming it deletes tubes, unless a later binary pass proves a direct delete/clear in that specific lifecycle.

Current Rust still lacks `TubeClass`, `tube_index`, `GetTubeAtCell`, or `TubeMovement` equivalents in `src/` by text scan.

## Open Questions

1. Retail map coverage still needs MIX extraction or a runtime map dump to prove how standard campaign/skirmish maps populate `[Tubes]`.
2. The exact path planner site that emits direction `8` for low bridge/tunnel routes should be traced when implementing pathfinding.
3. Hover/Mech/Ship producer sites should be line-by-line decompiled if those locomotor classes are implemented before tube traversal.
4. The low bridge damage/repair surface walkers still need a separate visual-state report if rendering parity is the next target.

## Sources

Ghidra:

- `DriveLocomotionClass::Process_Drive_Track @ 0x004B0F20`
- `WalkLocomotionClass::ProcessMovement @ 0x0075AEC0`
- `UnitClass::TubeMovement @ 0x007359F0`
- infantry tube movement routine around `0x0051B350`
- `FUN_00728280`
- `FUN_007283C0`
- direct byte-pattern searches for `+0x684`, `+0x685`, and `CellClass+0x116` writes

Docs:

- `LOW_BRIDGE_TUBECLASS_GHIDRA_REPORT.md`
- `LOW_BRIDGE_TUBECLASS_DOC_VERIFICATION.md`

