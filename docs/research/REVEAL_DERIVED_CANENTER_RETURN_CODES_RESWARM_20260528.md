# Reveal Derived CanEnter Return Codes - Reswarm Report

**Address(es):** `ObjectClass::Reveal @ 0x005F4EC0`; `vtable+0x1AC` dispatch targets `0x004264C0`, `0x004D9C10`, `0x0073F0A0`, `0x0051BF90`, `0x00449440`, `0x00415B10`, `0x0071C4D0`  
**Investigation Mode:** coverage-map, bounded representative class slice  
**Claimed Scope:** map key derived `vtable+0x1AC` CanEnter implementations reachable from ordinary active-YR reveal/unlimbo-style callers for Object/Techno/Foot/Unit/Infantry/Aircraft/Building/Bullet/Anim/Terrain, and confirm return-code polarity at the `ObjectClass::Reveal` call site.  
**Non-Scope:** full Unit/Infantry/Building/Terrain passability decision trees; every ObjectClass-derived class not named here; map-editor reveal bypass; pathfinder cost semantics except where needed to interpret return codes.  
**Confidence:** High for reveal polarity, arguments, vtable slot bindings, and stub/building/aircraft/terrain top-level return domains; Medium for broad liveness ranking because caller taxonomy is based on existing verified reports plus targeted spot checks, not a fresh exhaustive xref audit.  
**Active in YR:** Yes for ordinary non-editor reveal/unlimbo and runtime spawn paths; conditional by concrete object class and caller.

## 0. Working Notes

**Target question:** Which derived `vtable+0x1AC` CanEnter implementations can `ObjectClass::Reveal @ 0x005F4EC0` dispatch to for ordinary active-YR reveal/unlimbo callers, and what return values admit or reject reveal at this call site?

**Non-goals:** Do not rediscover base `Reveal` ordering; do not fully re-document Unit/Infantry pathfinding; do not edit stale docs; do not implement Rust.

**Evidence needed to mark COMPLETE:** decompile plus disassembly range for the base `Reveal` call/branch; vtable-slot memory reads for the bounded class set; function-body evidence for return domains; constructor/vtable ownership evidence for material classes; prior verified liveness evidence for ordinary YR callers.

**Stop conditions:** Stop after the representative active stock classes are mapped and return polarity is proven. If a function boundary is missing or derived class set expands beyond scope, record Remaining Uncertainty instead of broadening.

## 1. Overview

`ObjectClass::Reveal` uses the same virtual slot that movement/pathfinding calls `Can_Enter_Cell`, but at reveal time it collapses every derived return code to a binary gate: `0` admits reveal and any nonzero return rejects reveal before object mutation. This matters because Unit/Infantry return rich 0-7 movement codes, Aircraft returns 0/1 with landing-pad side effects, Building and Terrain return 0/7, and Object/Techno/Bullet/Anim inherit an unconditional `0` stub.

Active ordinary reveal/unlimbo callers therefore must not model this as a Rust boolean named "can enter" with `true == nonzero`. The native call site treats nonzero as failure.

## 2. Call-Site Polarity And Arguments

| Claim | Verified behavior | Evidence | Active in YR |
|---|---|---|---|
| Argument order | `CanEnter(cell, -1, -1, 0, 0)` after `CellClass::Get_Cell_At(coord)` | decompile `ObjectClass::Reveal @ 0x005F4EC0`; disassembled range `0x005F4F1B..0x005F4F49` | Yes, ordinary non-editor reveal |
| Return polarity | `return != 0` immediately returns reveal failure `0`; only `return == 0` proceeds to clear limbo/write coords/mark | same call branch in `0x005F4F1B..0x005F4F49` | Yes, ordinary non-editor reveal |
| Mutation ordering | This gate is before `InLimbo=0`, `NeedsRedraw=0`, raw coords, `Mark(MARK_PUT)`, display, logic registration | `OBJECTCLASS_REVEAL_EXACT_ORDERING_RESWARM_20260528.md`; fresh decompile `0x005F4EC0` | Yes |
| Editor bypass | `g_MapEditorMode != 0` skips this call entirely | decompile `0x005F4EC0` | Conditional; not ordinary gameplay |

## 3. Bounded Derived Slot Map

All slot values below are direct reads from each vtable at `base+0x1A8..base+0x1B7` unless noted.

| Concrete class | Vtable base | `+0x1AC` target | Return domain relevant to Reveal | Reveal interpretation | Class ownership / liveness evidence |
|---|---:|---:|---|---|---|
| `ObjectClass` | `0x007EF060` | `0x004264C0` shared zero stub | always `0` | always admits this gate | vtable symbol; read `0x007EF208` -> `... 0x004264C0 ...`; base object reveal is inherited by non-overriders |
| `TechnoClass` | `0x007F4960` | `0x004264C0` shared zero stub | always `0` | always admits this gate for abstract Techno-level dispatch | constructor decompile `0x006F2B40` writes `vtable__TechnoClass`; read `0x007F4B08` -> `0x004264C0`; ordinary concrete Foot/Building override later |
| `FootClass` | `0x007E8C94` | `0x004D9C10` | `0`, or locomotor COM result when `this+0x674 != 0` and 5th arg byte nonzero | `0` admits; any locomotor nonzero rejects | constructor decompile `0x004D31E0` writes `vtable__FootClass`; read `0x007E8E3C`; base Foot is not a normal stock concrete object |
| `UnitClass` | `0x007F5C70` | `0x0073F0A0` | movement codes `0..7` | only code `0` admits reveal; codes `1..7` reject | vtable read `0x007F5E18`; prior high-confidence Unit CanEnter report; active for vehicle unlimbo/reveal |
| `InfantryClass` | `0x007EB058` | `0x0051BF90` | movement codes including `0,1,2,3,5,6,7` in the verified body | only code `0` admits reveal; every nonzero soft/hard code rejects | vtable read `0x007EB200`; decompile `0x0051BF90`; active for infantry unlimbo/reveal |
| `AircraftClass` | `0x007E21F8` | `0x00415B10` | `0` or `1`; body scans 8 neighboring cells and may command the target object to move | `0` admits reveal; `1` rejects reveal at this call site | vtable read `0x007E23A0`; decompile `0x00415B10`; active for aircraft/landing interactions, but ordinary aircraft reveal with `target == -1` remains not runtime-sampled here |
| `BuildingClass` | `0x007E3EBC` | `0x00449440` | `0` or `7` from placement/foundation checks | `0` admits; `7` rejects | constructor/vtable evidence in prior bridge report plus read `0x007E4064`; decompile `0x00449440`; active for building placement/reveal probes |
| `BulletClass` | `0x007E46E4` | `0x004264C0` shared zero stub | always `0` | always admits this gate | constructor decompile `0x00466400` writes `vtable__BulletClass`; read `0x007E4890` at slot; bullet fire path calls `ObjectClass::Reveal` |
| `AnimClass` | `0x007E3354` | `0x004264C0` shared zero stub | always `0` | always admits this gate | constructor decompile `0x00421EA0` writes `vtable__AnimClass` and calls `ObjectClass::Reveal`; read `0x007E34FC` |
| `TerrainClass` | `0x007F522C` | `0x0071C4D0` | `0` or `7` | `0` admits; `7` rejects | vtable read `0x007F53D4`; bytes at `0x0071C4D0` show valid function prologue but Ghidra has no function boundary; decoded bytes show `return 7` on blocker and `return 0` on sentinel/end |

## 4. Return-Code Notes By Body

### Shared zero stub `0x004264C0`

Active for Object/Techno/Bullet/Anim in this bounded set. Decompile shows an unconditional `return 0`; disassembled range `0x004264C0..0x004264CF` was checked. In `Reveal`, this means the CanEnter gate cannot block these classes.

### FootClass `0x004D9C10`

The body returns `0` unless both `this+0x674` is non-null and the fifth stack argument byte is nonzero. Only then it calls the locomotor COM slot at `locomotor_vtable+0x1C` using a value read from `arg_cell+0x24`. At the base `Reveal` call site the fifth argument is `0`, so FootClass's own body returns `0` for reveal. Active in YR: base FootClass itself is not a normal stock concrete object; Unit/Infantry/Aircraft override the slot.

### UnitClass `0x0073F0A0`

The Unit body returns the full 0-7 movement passability code set. Prior verified reports map `0` as clear/passable and `7` as impassable, with soft blocker codes in between. For `Reveal`, those soft codes are not soft: `1`, `2`, `3`, `4`, `5`, `6`, and `7` all reject reveal because the caller checks only nonzero.

### InfantryClass `0x0051BF90`

The Infantry body mirrors the Unit-style 0-7 convention with infantry-specific branches, including tube/bridge checks, an early high-path acceptance branch, garrison logic, hostile-cell weapon-range gating, and final soft/hard blocker returns. For `Reveal`, only `0` admits. The garrison player-sell traces independently confirm a live non-reveal call site that also accepts only return `0` for infantry placement.

### AircraftClass `0x00415B10`

This is not a ground A* predicate. It loops over 8 offsets, calls another object's `+0x1AC` for a neighboring cell with last argument `1`, and if it can command the target to move, returns `1`; otherwise it returns `0`. In `Reveal`, that means the apparent "successfully bumped blocker" value `1` is a reveal failure. Active in YR: yes for aircraft landing/bump logic; exact behavior when `ObjectClass::Reveal` passes `-1,-1,0,0` into this aircraft-specific target-shaped body is not runtime-sampled here.

### BuildingClass `0x00449440`

BuildingClass converts placement/foundation passability to exactly `0` or `7`: passable/CanPlace true returns `0`, false returns `7`. At `Reveal`, `7` rejects before mutation. Active in YR: yes for building placement and building reveal probes.

### TerrainClass `0x0071C4D0`

Ghidra has no function boundary at `0x0071C4D0`; this report did not create one. Read-only byte inspection plus local x86 decoding shows a normal prologue, calls through `this->vtable+0x108`, loops over coordinate offsets from the type, calls cell/blocker helpers with either code `5` or `1` depending on type byte `+0x2B0`, returns `7` on failed helper, and returns `0` on sentinel/end. Active in YR: yes for map `[Terrain]` construction/unlimbo per `TIBTRE_SOURCE_OVERLAY_TYPE_REACHABILITY_AFTER_UNLIMBO_GHIDRA_REPORT.md`.

## 5. Current Rust Implementation Status

| Rust surface | Current shape | Delta |
|---|---|---|
| `src/sim/world/world_spawn.rs` | map/runtime spawn inserts entities, registers live objects, and adds occupancy directly (`self.entities.insert`, `register_live_object`, `occupancy.add`) | Missing native base `Reveal` gate API and class-specific `CanEnter` result collapse |
| `src/sim/world/mod.rs::register_live_object` | live-order registration is independent of reveal success | Should be reached only after native reveal/mark success and native alive/type gates |
| `src/sim/aircraft/drop_payload.rs` | has an impassable retry path that restores passenger before occupancy, plus a test comment for failed placement | Directionally aligned for "blocked means no unlimbo", but it uses path-grid/subcell checks, not class `+0x1AC` return semantics |
| `src/sim/production/production_sell.rs` | has local `garrison_infantry_can_enter_cell` approximation | Existing traces already mark it as an Infantry CanEnter approximation, not exact native code |

## 6. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| Base `Reveal` CanEnter arguments and polarity | verified | decompile `0x005F4EC0`; disassembly range `0x005F4F1B..0x005F4F49` | none |
| Object/Techno/Bullet/Anim shared stub | verified | vtable reads; decompile `0x004264C0`; constructor decompiles for Techno/Bullet/Anim | none for bounded classes |
| FootClass `+0x1AC` | verified for reveal args | vtable read; decompile `0x004D9C10`; `Reveal` passes fifth arg `0` | concrete stock liveness of base Foot not expected but not exhaustively proven |
| UnitClass `+0x1AC` | touched-not-exhausted | vtable read; decompile `0x0073F0A0`; prior Unit report | full branch tree not re-audited in this slot |
| InfantryClass `+0x1AC` | touched-not-exhausted | vtable read; decompile `0x0051BF90`; prior bridge report and traces | full branch tree not re-audited in this slot |
| AircraftClass `+0x1AC` | verified top-level return domain | vtable read; decompile `0x00415B10` | runtime sample of `Reveal` calling this with `-1,-1,0,0` |
| BuildingClass `+0x1AC` | verified top-level return domain | vtable read; decompile `0x00449440` | full type/foundation helper internals |
| TerrainClass `+0x1AC` | touched-not-exhausted | vtable read; bytes/decode `0x0071C4D0`; no Ghidra function boundary | helper identities and full branch names |
| Other ObjectClass-derived classes | deferred | not in bounded set | Overlay/Smudge/ParticleSystem/Particle/VoxelAnim/LightSource/etc. slot map if parent requests |

## 7. Open Questions - Final State

- `[RESOLVED] OQ-01 - What exactly is asked? -> derived `vtable+0x1AC` CanEnter return codes relevant to base `Reveal`, not base reveal ordering.` (evidence: parent OQ-REVEAL-014 and `OBJECTCLASS_REVEAL_EXACT_ORDERING_RESWARM_20260528.md`)
- `[RESOLVED] OQ-02 - What arguments does Reveal pass? -> `(cell, -1, -1, 0, 0)`.` (evidence: `0x005F4F1B..0x005F4F49`)
- `[RESOLVED] OQ-03 - What polarity does Reveal use? -> `0` admits, nonzero rejects before mutation.` (evidence: `0x005F4F1B..0x005F4F49`)
- `[RESOLVED] OQ-04 - Does map editor use this gate? -> no, editor mode skips it.` (evidence: decompile `0x005F4EC0`)
- `[RESOLVED] OQ-05 - Which representative classes share the zero stub? -> Object/Techno/Bullet/Anim in the bounded set.` (evidence: vtable reads and decompile `0x004264C0`)
- `[RESOLVED] OQ-06 - Does Foot block reveal? -> not for base Reveal's fifth arg `0`; it returns `0`.` (evidence: decompile `0x004D9C10`; reveal args)
- `[RESOLVED] OQ-07 - How do Unit/Infantry soft codes behave at Reveal? -> every nonzero soft code rejects reveal.` (evidence: decompile `0x0073F0A0`, `0x0051BF90`; reveal branch)
- `[RESOLVED] OQ-08 - Does Building return only 0/7? -> yes in top-level body.` (evidence: decompile `0x00449440`)
- `[RESOLVED] OQ-09 - Does Aircraft return 1 as success or failure at Reveal? -> body uses `1` for a successful bump/move command, but Reveal treats `1` as failure.` (evidence: decompile `0x00415B10`; reveal branch)
- `[RESOLVED] OQ-10 - Is Terrain mapped? -> slot target `0x0071C4D0` returns 0/7 in decoded bytes, but boundary is missing.` (evidence: read_memory/decode `0x0071C4D0`)
- `[DEFERRED] OQ-11 - What about Overlay/Smudge/Particle/VoxelAnim/LightSource and every other derived class?` (category: `bounded-cost-too-high`; reason: outside requested representative set; next-step-if-pursued: read every ObjectClass-derived vtable `+0x1AC` and classify reveal caller liveness.)
- `[DEFERRED] OQ-12 - What exact helpers does Terrain `0x0071C4D0` call?` (category: `bounded-cost-too-high`; reason: function boundary missing and top-level return polarity is enough for this slice; next-step-if-pursued: bounded Terrain CanEnter report with manual function-boundary recovery.)

## 8. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Proposed test name | Risk / do-not-do |
|---|---|---|---|---|---|---|---|
| Base reveal calls class `CanEnter(cell,-1,-1,0,0)` before mutation and admits only return `0`. | `0x005F4F1B..0x005F4F49` | missing | `src/sim/world/world_spawn.rs`; future reveal/unlimbo API | Add a reveal gate that maps native class return codes to `return == 0` admission before coords/occupancy/live registration. | A limbo unit whose native-style CanEnter returns code `2` remains in limbo with coords/occupancy/live order unchanged. | `reveal_rejects_nonzero_canenter_soft_code_before_mutation` | Do not treat soft movement codes as successful placement. |
| Object/Techno/Bullet/Anim use shared zero stub at `+0x1AC`; their reveal gate does not block on CanEnter. | vtable reads; decompile `0x004264C0`; Bullet/Anim constructors and reveal callers | unchecked/missing class dispatch | spawn/reveal code for bullets/anims/effects | Route these classes through a zero-code CanEnter unless a more-derived class overrides it. | Bullet fire reveal and non-bouncer Anim constructor reveal do not consult path-grid blockage and can proceed to later Mark/reveal stages. | `reveal_zero_stub_classes_bypass_cell_passability_gate` | Do not apply ground path-grid blocking to bullets/anims at the base Reveal CanEnter gate. |
| Building and Terrain `+0x1AC` return only `0` or `7`; `7` rejects reveal. | decompile `0x00449440`; vtable/bytes `0x0071C4D0`; terrain liveness docs | missing exact placement/reveal split | building spawn/deploy, terrain map load/unlimbo | Preserve `0/7` polarity and keep class-specific placement/terrain checks separate from Unit/Infantry movement soft-code handling. | Blocked building/terrain reveal returns failure before occupancy/live registration; allowed returns success. | `reveal_building_terrain_code7_blocks_unlimbo` | Do not reuse Unit/Infantry soft-code meanings for Building/Terrain. |

## 9. Negative Facts / Do Not Do

- Do not implement `CanEnter` as `true == nonzero` for reveal. Evidence: `ObjectClass::Reveal @ 0x005F4F1B..0x005F4F49` rejects nonzero.
- Do not let Unit/Infantry soft movement codes (`1..6`) admit reveal. Evidence: `UnitClass::Can_Enter_Cell @ 0x0073F0A0` and `InfantryClass @ 0x0051BF90` return those codes, but reveal checks only `!= 0`.
- Do not path-grid-block Bullet/Anim/Object/Techno reveal at this gate. Evidence: their bounded `+0x1AC` slot is shared `0x004264C0`, unconditional `0`.
- Do not treat AircraftClass `Can_Enter_Cell` return `1` as reveal success. Evidence: `0x00415B10` returns `1` after commanding a move, but reveal rejects any nonzero.
- Do not assume `vtable+0x1B0` is the reveal gate. Evidence: `Reveal` calls `+0x1AC`; `+0x1B0` is a bridge sub-check for Unit/Infantry/Foot and unrelated/inherited for Aircraft/Building.

## 10. Remaining Uncertainty

- Full ObjectClass-derived slot map outside Object/Techno/Foot/Unit/Infantry/Aircraft/Building/Bullet/Anim/Terrain remains unexamined.
- TerrainClass `0x0071C4D0` lacks a Ghidra function boundary; this report used read-only memory decode and stops at top-level 0/7 polarity, not helper naming.
- AircraftClass reveal-liveness with the exact `Reveal` argument tuple was not runtime-sampled; the handler body is active in YR, but its ordinary reveal use may be rare or absent for stock aircraft.

## 11. Stale Docs / Replacement Wording

`C:/Users/enok/Documents/ra2-rust-game/docs/research/LIMBO_AND_CELL_OCCUPATION_LIFECYCLE_GHIDRA_REPORT.md` should replace any wording that implies a boolean "CanEnter true admits reveal" with:

> In normal non-editor mode, `ObjectClass::Reveal @ 0x005F4EC0` calls the object's `vtable+0x1AC` handler as `CanEnter(cell,-1,-1,0,0)` before any object mutation. At this call site, return `0` is the only admitting value; any nonzero value rejects reveal and leaves limbo/coords/occupancy/live registration untouched. Derived handlers are not uniform booleans: Unit/Infantry return movement codes where `1..7` all reject reveal, Building/Terrain return `0/7`, Aircraft returns `0/1`, and Object/Techno/Bullet/Anim inherit an unconditional `0` stub.

## Sources

- Ghidra read-only decompile/disassembly:
  - `ObjectClass::Reveal @ 0x005F4EC0`; disassembled `0x005F4F1B..0x005F4F49`
  - `AnimClass__Receive_stub @ 0x004264C0`; disassembled `0x004264C0..0x004264CF`
  - `FootClass__LocomotorPassabilityCheck @ 0x004D9C10`
  - `UnitClass__Can_Enter_Cell @ 0x0073F0A0`; disassembled `0x0073F0A0..0x0073F19F`
  - `FUN_0051BF90` InfantryClass `+0x1AC`; disassembled `0x0051BF90..0x0051C08F`
  - `FUN_00449440` BuildingClass `+0x1AC`
  - `AircraftClass__Can_Enter_Cell @ 0x00415B10`
  - `TerrainClass +0x1AC target @ 0x0071C4D0` read-memory/decode; Ghidra function boundary missing
  - Constructors `TechnoClass @ 0x006F2B40`, `FootClass @ 0x004D31E0`, `BulletClass @ 0x00466400`, `AnimClass @ 0x00421EA0`
- Ghidra vtable slot reads:
  - `0x007EF208`, `0x007F4B08`, `0x007E8E3C`, `0x007F5E18`, `0x007EB200`, `0x007E23A0`, `0x007E4064`, `0x007E4890`, `0x007E34FC`, `0x007F53D4`
- Prior research:
  - `docs/research/OBJECTCLASS_REVEAL_EXACT_ORDERING_RESWARM_20260528.md`
  - `docs/research/bridges/03-traversal-pathfinding-entry/BRIDGE_CAN_ENTER_CELL_HIERARCHY_GHIDRA_REPORT.md`
  - `docs/research/UNIT_CAN_ENTER_CELL_GHIDRA_REPORT.md`
  - `docs/research/BULLETCLASS_INIT_AND_FIRE_GHIDRA_REPORT.md`
  - `docs/research/TIBTRE_SOURCE_OVERLAY_TYPE_REACHABILITY_AFTER_UNLIMBO_GHIDRA_REPORT.md`
  - `docs/research/traces/PLAYER_SELL_CAN_ENTER_CELL_RESIDUAL_POSTFIX_TRACE.md`
- Rust surfaces scanned:
  - `src/sim/world/world_spawn.rs`
  - `src/sim/world/mod.rs`
  - `src/sim/aircraft/drop_payload.rs`
  - `src/sim/production/production_sell.rs`
