# FootClass+0x68B Bridge-State Mismatch Consumers - Ghidra Research Report

**Address(es):** primary field sites `0x004D33BA`, `0x004B3391`, `0x004B45ED`, `0x00515513`, `0x0051BA94`, `0x005B094F`, `0x006A29E0`, `0x006A3C19`, `0x00736038`, `0x0075B662`, reader `0x004DBD0C`  
**Investigation Mode:** exhaustive-slice  
**Claimed Scope:** direct live readers/writers of `FootClass+0x68B` and consumer effects relevant to bridge-state mismatch/low-bridge traversal.  
**Non-Scope:** generic bridge pathfinding, bridge repair/damage body rendering, and non-`+0x68B` bridge layer consumers.  
**Confidence:** High for direct field readers/writers and no gameplay consumer found; Medium for save/load persistence boundary because save/load uses generic object/base serialization around this block.  
**Active in YR:** Conditional. Writers are active when their locomotor/path branch runs; the only direct reader found is checksum/state serialization, not gameplay.

## 1. Overview

`FootClass+0x68B` is a sticky byte flag initialized to `0`, set to `1` by several locomotor movement/tube paths, and directly read only by `FootClass__ComputeChecksum`. I found no direct gameplay consumer that reads `+0x68B` to trigger repath, stutter, movement abort, renderer layer switching, audio, or bridge repair/damage behavior.

For the low-bridge infantry case: the Walk locomotor mismatch detector is real and active in YR, and it sets `+0x68B` when the next-step cell's structural bridge bit (`CellClass+0x140 & 0x100`) disagrees with `FootClass+0x8C` (`on_bridge`). Under the parent-settled low-bridge condition where `on_bridge` remains `0` while cells may still carry bridge flags, this means low-bridge traversal can set the byte every affected step, but the byte itself has no verified downstream gameplay effect.

## 2. Class Layout / Key Offsets

| Offset | Type | Purpose in this slice | Active in YR |
|---|---:|---|---|
| `FootClass+0x68A` | byte | adjacent movement/stuck-sound flag; often cleared near movement completion | Yes, but separate from this target |
| `FootClass+0x68B` | byte | sticky bridge/path transition marker; initialized `0`, set `1`, not cleared by direct writer found | Conditional: writer branches only |
| `FootClass+0x8C` | byte | `on_bridge` runtime bridge-state flag consumed by mismatch comparisons | Yes |
| `CellClass+0x140 bit 0x100` | bit | structural bridge cell flag used by mismatch tests | Yes |
| `FootClass+0x684/+0x685` | bytes | tube id/cursor fields adjacent to unit/infantry tube movement | Conditional: tube movement only |

## 3. Core Logic

Direct offset search used the little-endian displacement `8B 06 00 00` plus byte-access encodings. Actual `+0x68B` instructions found:

| Site | Function | Access | Condition/effect | Active in YR |
|---|---|---|---|---|
| `0x004D33BA` | `FootClass__Constructor` | write `0` | initializes field to `0` with adjacent `0x685..0x691` bytes | Yes: all FootClass-derived units |
| `0x004B3391` | `DriveLocomotionClass__Process_Movement` | write `1` | next cell bridge bit differs from `on_bridge`, before vtable `+0x29C` step gate | Yes: ground vehicle Drive locomotor |
| `0x004B45ED` | `DriveLocomotionClass__Process_Movement` | write `1` | after path queue shift and `FootClass+0x638=-1`, before common continuation | Yes when that blocked/shift branch runs |
| `0x006A29E0` | `ShipLocomotionClass__Process_Movement` | write `1` | same mismatch pattern as Drive | Yes: naval Ship locomotor |
| `0x006A3C19` | `ShipLocomotionClass__Process_Movement` | write `1` | same path queue shift branch shape as Drive | Yes when that branch runs |
| `0x0075B662` | `WalkLocomotionClass__ProcessMovement` | write `1` | `((cell.flags >> 8) & 1) != FootClass+0x8C` | Yes: infantry Walk locomotor |
| `0x00736038` | `UnitClass__TubeMovement` | write `1` | tube exit/finalization after clearing `+0x684`, enabling `+0x124`, calling `+0x18C(2)` | Conditional: unit tube movement |
| `0x0051BA94` | `FUN_0051B350` | write `1` | infantry-style tube finalization: clears `+0x684` then calls vtable `+0x18C(2)` | Conditional: infantry tube movement |
| `0x00515513` | `FUN_00514F70` | write `1` | Hover movement mismatch pattern (`cell.flags & 0x100` versus `on_bridge`) | Yes for Hover locomotor units |
| `0x005B094F` | `FUN_005B01C0` | write `1` | Jumpjet movement mismatch-like bridge/on_bridge comparison | Yes for JumpJet units |
| `0x004DBD0C` | `FootClass__ComputeChecksum` | read | includes the byte in deterministic checksum stream between `0x68A` and `0x68C` | Yes for checksum paths |

No direct `cmp/test/movzx/mov` gameplay reader of `+0x68B` was found outside checksum. Search false positives included branch-immediate bytes and `WinMain` heap-pool class id registration (`PUSH 0x68B`), not `FootClass+0x68B`.

## 4. INI Keys

No INI key directly gates `FootClass+0x68B`. Activity comes from locomotor bindings and map cell/tube data.

| Data source | Effect | Active in YR |
|---|---|---|
| Unit/Infantry locomotor CLSIDs in `rulesmd.ini`/base rules | determine whether Drive/Ship/Walk/Hover/Jumpjet paths can run | Yes, per stock units |
| Map `CellClass+0x140` bridge flags and `CellClass+0x116` tube index | feed mismatch/tube branches | Yes on bridge/tube maps |
| Low bridge cells (`LandType == 10`, tube-backed) | trigger the parent low-bridge mismatch scenario when structural bridge flag and `on_bridge` disagree | Conditional by map/cell |

## 5. Integration Points / Consumer Chain

The writer chain is movement-local: locomotor detects mismatch or tube transition, writes `+0x68B=1`, and continues its normal movement branch. The byte is not passed to `FootClass__Find_Path`, `Can_Enter_Cell`, scatter, sound playback, bridge state repair/damage walkers, or renderer layer selection by any direct read found.

The only direct consumer verified is `FootClass__ComputeChecksum @ 0x004DBAD0`, where `0x004DBD0C` reads the byte and feeds `0x004A1CA0`. Active in YR: Yes for checksum/determinism paths; this is not a player-visible movement effect.

`FootClass__Load @ 0x004DB3C0` and `FootClass__Save @ 0x004DB690` do not directly name `0x68B`; they call base/object serialization helpers then handle dynamic vectors and locomotor persistence. Active in YR: Yes for save/load, but no special `+0x68B` gameplay consumer is present there.

## 6. Current Rust Implementation Status

Rust has no `GameEntity` field equivalent to `FootClass+0x68B` (`src/sim/game_entity.rs:132-138` only models `bridge_occupancy` and `on_bridge`; `src/sim/game_entity.rs:155-157` models low-bridge tube state). Current low-bridge tube finish logic projects low-bridge tube cells into `on_bridge=true` via `src/sim/movement/tube_movement.rs:262-270`, which conflicts with the parent-settled observation that low bridges can keep `on_bridge=0` while bridge flags remain set.

Because the binary byte has no gameplay reader, Rust does not need a repath/stutter/audio/renderer consumer for `+0x68B`. Rust may need a deterministic/debug-only marker only if world hashing/save parity explicitly requires it later.

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| Constructor initialization | verified | `0x004D33BA` | none |
| Drive writers | verified | `0x004B3391`, `0x004B45ED` | exact branch naming can be improved in a Drive-only doc |
| Ship writers | verified | `0x006A29E0`, `0x006A3C19` | none for `+0x68B` |
| Walk low-bridge/mismatch writer | verified | `0x0075B662`; parent trace `PATHFIND_INFANTRY_LOW_BRIDGE_RAMP_TRACE.md` | runtime low-bridge `on_bridge=0` confirmation belongs to parent trace |
| Unit/Infantry tube finalization writers | verified | `0x00736038`, `0x0051BA94` | broader tube movement behavior out of scope |
| Hover/Jumpjet writers | verified | `0x00515513`, `0x005B094F` | exact visible branch effect out of scope; no `+0x68B` consumer found |
| Direct gameplay consumers | verified absent in this slice | direct byte-pattern searches for `0x68B` access; only checksum read at `0x004DBD0C` | a dynamic watchpoint could confirm no indirect computed-address read at runtime |
| Checksum consumer | verified | `0x004DBD0C` in `FootClass__ComputeChecksum` | none |
| Save/load persistence | touched-not-exhausted | `0x004DB3C0`, `0x004DB690` | generic base serializer details, if savegame byte-level parity is needed |
| Current Rust surfaces | verified for absence of field | `src/sim/game_entity.rs:132-138`, `src/sim/movement/tube_movement.rs:262-270` | no Rust edits in this slot |

## 8. Open Questions - Final State

- `[RESOLVED] OQ-1 - Does low-bridge/Walk traversal write +0x68B? -> Yes, Walk writes when `CellClass+0x140 bit 0x100` differs from `FootClass+0x8C`.` (evidence: `0x0075B662`; trace doc lines in `PATHFIND_INFANTRY_LOW_BRIDGE_RAMP_TRACE.md`)
- `[RESOLVED] OQ-2 - Are there movement/repath consumers? -> No direct gameplay reader found; `FootClass__Find_Path`, blocked/stuck branches, and movement abort paths do not read `+0x68B`.` (evidence: direct offset-access search; reader list only `0x004DBD0C`)
- `[RESOLVED] OQ-3 - Is there an audio/visual consumer? -> No direct audio or renderer reader found. Adjacent `+0x68A` triggers sound in movement; `+0x68B` does not.` (evidence: `0x004B2E68`/movement decompile context for `+0x68A`; no `+0x68B` reads outside checksum)
- `[RESOLVED] OQ-4 - Is the Ship doc renderer claim valid? -> No, stale. The direct reader search found no renderer read; only checksum reads `+0x68B`.` (evidence: `SHIP_LOCOMOTION_CLASS_GHIDRA_REPORT.md:751`; `0x004DBD0C`)
- `[RESOLVED] OQ-5 - Are `0x00515D3A` and `0x005B0BD4` +0x68B writers? -> No, both write `+0x6B7`, not `+0x68B`.` (evidence: assembly contexts `0x00515D3A`, `0x005B0BD4`)
- `[RESOLVED] OQ-6 - Does the byte reset after being set? -> No direct writer-to-zero found except constructor initialization.` (evidence: `0x004D33BA`; zero-write pattern search)
- `[DEFERRED] OQ-7 - Could generic save/load serialize the byte by a class field table?` (category: `requires-different-system-context`; reason: save/load helpers are generic and not needed for movement consumer parity; next-step-if-pursued: byte-level savegame serializer audit)
- `[DEFERRED] OQ-8 - Could an indirect computed-address runtime read evade static offset search?` (category: `needs-runtime-debugger`; reason: no static direct reader found, but dynamic watchpoint is the exhaustive proof; next-step-if-pursued: set read watchpoint on a live FootClass instance `+0x68B` during low-bridge traversal)

## 9. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| `+0x68B` has no verified movement/repath/stutter/audio/render consumer; only checksum reads it | `0x004DBD0C`; no other direct readers from offset search | Rust lacks the field; acceptable unless deterministic/save parity explicitly needs it | `src/sim/game_entity.rs`, `src/sim/world/world_hash.rs` | Do not add gameplay side effects from this byte | Low-bridge infantry crosses a bridge-flagged low bridge cell with no extra repath/stutter/audio caused solely by mismatch marker -> `test_low_bridge_mismatch_flag_consumer_noops` | Do not implement guessed repath or renderer logic |
| Walk mismatch detector writes `+0x68B=1` when structural bridge bit differs from `on_bridge` | `0x0075B662` | field missing; no gameplay delta if no consumer | optional debug/determinism state only | If modeled, set sticky marker but leave movement unchanged | Deterministic trace can record mismatch marker while path index and movement phase stay unchanged -> `test_walk_bridge_mismatch_marker_sets_without_repath` | Do not clear every tick unless a future save/checksum audit proves a reset |
| Low-bridge Rust currently projects tube cells into `on_bridge=true`; parent evidence says low bridge can leave `on_bridge=0` while still setting mismatch | `src/sim/movement/tube_movement.rs:262-270`; parent trace | probable mismatch, but this slot only confirms no `+0x68B` consumer | `src/sim/movement/tube_movement.rs`, `src/sim/movement/movement_bridge.rs` | Future low-bridge fix should preserve no-op `+0x68B` semantics while correcting `on_bridge` if needed | Low-bridge tube traversal keeps intended low-bridge `on_bridge` behavior and does not introduce mismatch-driven repath -> `test_low_bridge_tube_on_bridge_policy_does_not_repath_from_mismatch_marker` | Do not use `+0x68B` as a shortcut to force bridge occupancy |

### Negative Facts / Do Not Do

- Do not implement `+0x68B` as a repath trigger. Evidence: no direct reader in `FootClass__Find_Path` caller chain; only checksum read `0x004DBD0C`.
- Do not implement `+0x68B` as a renderer layer switch. Evidence: stale renderer claim in `SHIP_LOCOMOTION_CLASS_GHIDRA_REPORT.md:751`; no renderer read found.
- Do not copy the stale addresses `0x00515D3A` / `0x005B0BD4` as `+0x68B` writes. Evidence: both write `FootClass+0x6B7`.
- Do not tie stuck sound to `+0x68B`. Evidence: sound branches read/write adjacent `+0x68A`, not `+0x68B`.
- Do not assume "low bridge mismatch" means standard high-bridge `on_bridge` should be forced true. Evidence: `+0x68B` is the mismatch marker, not the occupancy flag; `+0x8C` remains the on-bridge source.

### Stale Docs / Follow-up Docs

- `C:/Users/enok/Documents/ra2-rust-game-docs/SHIP_LOCOMOTION_CLASS_GHIDRA_REPORT.md:751` replacement wording: "`techno+0x68B` is set by Ship movement bridge/mismatch branches, but this investigation found no direct renderer or gameplay reader; the only verified direct read is `FootClass__ComputeChecksum @ 0x004DBD0C`."
- `C:/Users/enok/Documents/ra2-rust-game-docs/UNIT_COLLISION_AND_REPATH_TRIGGERS_GHIDRA_REPORT.md:420-421` replacement wording: "`0x00515D3A` and `0x005B0BD4` write `FootClass+0x6B7`, not `+0x68B`; the nearby verified `+0x68B` writes are `0x00515513` and `0x005B094F`."
- `C:/Users/enok/Documents/ra2-rust-game-docs/BRIDGE_LOCOMOTOR_WALK_DROPPOD_TELEPORT_GHIDRA_REPORT.md:213` replacement wording: "Purpose now traced: no direct gameplay consumer found; treat `+0x68B` as a sticky checksum/debug/state marker unless a dynamic watchpoint later proves an indirect runtime read."

## Sources

- Ghidra decompiled/read-only: `0x004D31E0`, `0x004B2630`, `0x006A1C80`, `0x0075AEC0`, `0x007359F0`, `0x00514F70`, `0x0051B350`, `0x005B01C0`, `0x004DBAD0`, `0x004DB3C0`, `0x004DB690`.
- Ghidra assembly contexts: `0x004D33BA`, `0x004B3391`, `0x004B45ED`, `0x00515513`, `0x0051BA94`, `0x005B094F`, `0x006A29E0`, `0x006A3C19`, `0x00736038`, `0x0075B662`, `0x004DBD0C`, `0x00515D3A`, `0x005B0BD4`.
- Docs referenced: `BRIDGE_LOCOMOTOR_WALK_DROPPOD_TELEPORT_GHIDRA_REPORT.md`, `BRIDGE_LOCOMOTOR_AIR_HOVER_GHIDRA_REPORT.md`, `SHIP_LOCOMOTION_CLASS_GHIDRA_REPORT.md`, `UNIT_COLLISION_AND_REPATH_TRIGGERS_GHIDRA_REPORT.md`, `traces/PATHFIND_INFANTRY_LOW_BRIDGE_RAMP_TRACE.md`.
- Rust scan: `src/sim/game_entity.rs`, `src/sim/movement/movement_bridge.rs`, `src/sim/movement/tube_movement.rs`, `src/sim/pathfinding/core.rs`, `src/sim/bridge_state/`.
