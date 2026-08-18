# Scatter / Bump-Crush RNG Classification - Ghidra Research Report

**Address(es):** `0x00481670`, `0x00743A50`, `0x0051D0D0`, `0x00481180`, `0x004DA530`, `0x0074177A`
**Investigation Mode:** coverage-map
**Claimed Scope:** RNG range/order classification for current Rust `src/sim/movement/scatter.rs` and `src/sim/movement/bump_crush.rs` scatter/sub-cell RNG calls.
**Non-Scope:** full movement parity, complete `Find_Nearby_Passable_Cell` internals, all direct Scatter callers outside movement/bump/crush, full crush damage behavior.
**Confidence:** High for listed RNG bounds and current Rust deltas; Medium for exact future Rust design shape.
**Active in YR:** Yes for all material binary paths listed here unless explicitly marked dormant in current Rust.

## 0. Target / Non-Goals / Evidence Gate / Stop Conditions

- Target question: classify whether current Rust `next_range_u32(8)` scatter starts and `next_range_u32(4)` sub-cell rotation match gamemd/YR RNG ranges and call order.
- Non-goals: do not patch Rust; do not re-audit every scatter caller; do not rename Ghidra symbols; do not decide full movement architecture.
- Evidence needed for COMPLETE: decompile plus assembly/xref evidence for `CellClass::Scatter_Objects`, `UnitClass::Scatter`, `InfantryClass::Scatter`, `PlaceInfantryInCell`, and current Rust call sites.
- Stop conditions: classify each in-scope Rust RNG call GREEN/YELLOW/RED; record unresolved deeper call-chain questions as deferred rather than expanding scope.

## 1. Overview

Current Rust has three in-scope RNG shapes: `scatter_blocker(...).next_range_u32(8)`, dormant `tick_idle_scatter(...).next_range_u32(8)`, and `allocate_sub_cell_with_preference(...).next_range_u32(4)`. The two scatter `0..8` starts are RED against gamemd: movement scatter enters `CellClass::Scatter_Objects`, which dispatches each occupant's virtual `Scatter`; `UnitClass::Scatter` null-coordinate movement scatter does not directly consume an 8-way RNG, and `InfantryClass::Scatter` uses `RandomRanged(0,4)` in its scatter-direction paths. The sub-cell rotation `next_range_u32(4)` is GREEN as a wrapper for `RandomRanged(0,3)` when quadrant 0 reaches `PlaceInfantryInCell`.

## 2. Current Rust Implementation Status

| Rust surface | Current RNG | Classification | Reason |
|---|---:|---|---|
| `src/sim/movement/bump_crush.rs::scatter_blocker` | `next_range_u32(8)` before neighbor scan | RED | gamemd cell scatter dispatches virtual `Scatter`; no one-size 8-way draw. |
| `src/sim/movement/scatter.rs::tick_idle_scatter` | `next_range_u32(8)` | RED if enabled; currently dormant | `world/mod.rs` comments it out; gamemd idle scatter calls vtable `Scatter` every 64-frame phase, not a local 8-neighbor RNG start. |
| `src/sim/movement/scatter.rs::scatter_units_from_cell` | no RNG | YELLOW | Building/foundation scatter details are outside this RNG slice; no direct mismatch found in scope. |
| `src/sim/movement/bump_crush.rs::allocate_sub_cell_with_preference` | `next_range_u32(4)` only for quadrant 0 | GREEN | `RandomRanged(0,3)` at `0x0048139A`; current wrapper `next_range_u32(4)` means inclusive `0..3`. |

## 3. Core Binary Logic

### 3.1 Cell scatter dispatch is virtual, not a local RNG pick

`CellClass__Scatter_Objects @ 0x00481670` selects either `cell+0xE4` or `cell+0xE8`, optionally does an eligibility pre-scan when `force == 0`, collects up to 10 occupants, and calls `occupant->vtable+0x174(coord, threat, force)` in collected order. Decompile shows no RNG draw inside `CellClass__Scatter_Objects`.

Assembly confirms active movement scatter sites pass `NullCoord`, `threat=1`, `force=1`, and a layer flag:

- Drive track scatter call `0x004B1F43`: `PUSH EDX` layer, `PUSH 0x1` force, `PUSH 0x1` threat, `PUSH 0x8A0790` NullCoord, `CALL 0x00481670`.
- Walk locomotion scatter call `0x0075B891`: `PUSH EDX` layer, `PUSH 0x1` force, `PUSH 0x1` threat, `PUSH 0xB45BE8` NullCoord, `CALL 0x00481670`.
- Crusher entry scatter call `0x0074177A`: bridge/ground variants push layer, `PUSH 0x0` force, `PUSH 0x1` threat, `PUSH 0xB1CFE8` NullCoord, `CALL 0x00481670`.

Active in YR: Yes. These are live locomotor/per-cell movement paths.

### 3.2 UnitClass scatter RNG shapes

`UnitClass__Scatter @ 0x00743A50` has three direct `RandomRanged` calls:

- `0x00743D2B`: assembly `PUSH 0x4`, `PUSH 0x1`, `LEA ECX,[Scenario+0x218]`, `CALL 0x0065C7E0`; this is `RandomRanged(1,4)` for the towed-target 1-in-4 allow case. Active in YR: conditional on `TowTarget != NULL` and `threat == false`.
- `0x00743DC5`: assembly `PUSH 0x2`, `PUSH 0`, call, then `LEA EDI,[EAX + dir - 1]`; this is `RandomRanged(0,2)-1` jitter for real threat-coordinate directional scatter. Active in YR: Yes for non-null scatter hints.
- `0x00743DFF`: same `RandomRanged(0,2)-1` after `RateTimer__Current`, for facing-based directional scatter branch. Active in YR: conditional.

No `RandomRanged(0,7)` appears in `UnitClass__Scatter`. Its null-coordinate branch calls `FootClass__Find_Nearby_Passable_Cell` after gates rather than drawing a local 8-way start.

### 3.3 InfantryClass scatter RNG shapes

`InfantryClass__Scatter @ 0x0051D0D0` has direct scenario RNG calls:

- `0x0051D2BA`: assembly `PUSH 0x4`, `PUSH EBP`, `LEA ECX,[Scenario+0x218]`, `CALL 0x0065C7E0`, then subtracts 2 from the reduced direction expression. With `EBP == 0` in this path, this is the documented `RandomRanged(0,4)` scatter-facing roll. Active in YR: Yes.
- `0x0051D385`: assembly `PUSH 0x4`, `PUSH EBP`, `LEA ECX,[Scenario+0x218]`, `CALL 0x0065C7E0`, then `LEA EAX,[EDI + EAX - 2]`. This is the second `RandomRanged(0,4)` scatter-direction path. Active in YR: Yes.

No in-scope evidence supports replacing these with `RandomRanged(0,7)`, raw `Next`, or `RandomRanged(1,4)`.

### 3.4 Sub-cell rotation RNG shape

`CellClass__PlaceInfantryInCell @ 0x00481180` calls `RandomRanged(0,3)` only for quadrant 0/NW-center random rotation:

- `0x0048139A`: assembly `PUSH 0x3`, `PUSH 0x0`, `LEA ECX,[Scenario+0x218]`, `CALL 0x0065C7E0`, then indexes `DAT_0081CC98 + EAX*4`.
- This happens after the placement logic decides to use the random rotation table. Static table bytes and prior GREEN audit: `DAT_0081CC98` rotations are `[1,2,3,4]`, `[2,3,4,1]`, `[3,4,1,2]`, `[4,1,2,3]`.

Current Rust `next_range_u32(4)` equals `RandomRanged(0,3)` after the `SimRng` rewrite. Active in YR: Yes, normal infantry placement.

### 3.5 FootClass idle scatter timing

`FootClass__AI @ 0x004DA530` decompile shows idle scatter when `(g_CurrentFrameCounter & 0x3f) == 0x3f`, `NavTarget == NULL`, not bridge cell, mission allows scatter, and height is 0, then calls `vtable+0x174` with zero coord and threat flag. Assembly at `0x004DAE59` shows the virtual call. Active in YR: Yes. Current Rust `tick_idle_scatter` is commented out in `src/sim/world/mod.rs`, so its `next_range_u32(8)` is dormant in current runtime.

## 4. INI Keys

| Key | File/default | Effect in this slice |
|---|---|---|
| `[CombatDamage] PlayerScatter=no` | `ini/rulesmd.ini:900` | Scatter eligibility gate in `CellClass__Scatter_Objects` and per-class scatter; standard YR player-owned units do not auto-scatter from threats by default. |
| `[IQ] Scatter=2` | `ini/rulesmd.ini:3164` | AI IQ threshold for scatter participation. |
| `[General] CloseEnough=2.25` | `ini/rulesmd.ini:58` | Locomotor blocked/scatter decision context, not a direct RNG bound. |
| `[General] BlockagePathDelay=60` | `ini/rulesmd.ini:3107` | Rust blocked-delay retry context, not a direct gamemd random range. |
| `Crushable`, `Crusher`, `OmniCrusher`, `OmniCrushResistant`, `DeployedCrushable` | many object sections | Determine crush/scatter path liveness; not direct random ranges. |

## 5. Integration Points

- `movement_occupancy.rs` calls `bump_crush::scatter_blocker` for `FriendlyStationary` blockers and delayed retry scatter.
- `world/mod.rs` currently comments out `scatter::tick_idle_scatter`, so its RNG call is not consumed in normal Rust ticks.
- `movement_reservation.rs` and `movement_step.rs` call `allocate_sub_cell_with_preference`, which can consume one `RandomRanged(0,3)` equivalent only for quadrant 0/NW-center placement.

## 6. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| `CellClass__Scatter_Objects` dispatch | verified | decompile `0x00481670`, assembly callers `0x004B1F43`, `0x0075B891`, `0x0074177A` | none for RNG classification |
| `UnitClass__Scatter` RNG bounds | verified | decompile `0x00743A50`, assembly `0x00743D2B`, `0x00743DC5`, `0x00743DFF` | full non-RNG gates not exhausted |
| `InfantryClass__Scatter` RNG bounds | verified | decompile `0x0051D0D0`, assembly `0x0051D2BA`, `0x0051D385`, RNG doc | exact source of `EBP==0` path remains tied to decompiler register state; bounds match docs/assembly |
| `PlaceInfantryInCell` rotation | verified | decompile `0x00481180`, assembly `0x0048139A`, `INFANTRY_SUBCELL_POSITIONING.md` | none for RNG classification |
| `FootClass__AI` idle scatter timing | verified | decompile `0x004DA530`, assembly `0x004DAE59` | Rust re-enable design out-of-scope |
| `Find_Nearby_Passable_Cell` internals | deferred | called from `UnitClass__Scatter`/`InfantryClass__Scatter` | needs dedicated path-search audit if implementing exact scatter destination selection |
| Current Rust `scatter_blocker` | verified | `src/sim/movement/bump_crush.rs:566`, `:595` | patch needed |
| Current Rust idle scatter | verified | `src/sim/movement/scatter.rs:71`, `:123`; dormant call in `src/sim/world/mod.rs:1434` | keep disabled or redesign before enabling |

## 7. Open Questions - Final State

- `[RESOLVED] OQ-1 - Does `CellClass::Scatter_Objects` itself consume RNG? -> No; it dispatches virtual Scatter calls.` (evidence: `0x00481670` decompile)
- `[RESOLVED] OQ-2 - Do movement scatter calls pass NullCoord and force? -> Drive/Walk use `force=1`; crusher per-cell entry uses `force=0`; all pass NullCoord in sampled active paths.` (evidence: `0x004B1F43`, `0x0075B891`, `0x0074177A`)
- `[RESOLVED] OQ-3 - Is Rust `next_range_u32(8)` a verified gamemd scatter bound? -> No for this scope.` (evidence: no `RandomRanged(0,7)` in `UnitClass__Scatter`; infantry uses `0,4`)
- `[RESOLVED] OQ-4 - Is `RandomRanged(1,4)` a direction roll? -> No; in `UnitClass__Scatter` it is the towed-target 1-in-4 gate.` (evidence: `0x00743D2B`)
- `[RESOLVED] OQ-5 - Is `RandomRanged(0,2)-1` present? -> Yes, UnitClass directional jitter only.` (evidence: `0x00743DC5`, `0x00743DFF`)
- `[RESOLVED] OQ-6 - Is `RandomRanged(0,4)` present? -> Yes, InfantryClass scatter-facing paths.` (evidence: `0x0051D2BA`, `0x0051D385`, `RNG_SYSTEM_GHIDRA_REPORT.md`)
- `[RESOLVED] OQ-7 - Is sub-cell `next_range_u32(4)` correct? -> Yes for `RandomRanged(0,3)` rotation table selection.` (evidence: `0x0048139A`)
- `[RESOLVED] OQ-8 - Is Rust idle scatter currently consuming RNG? -> No, world tick call is commented out.` (evidence: `src/sim/world/mod.rs:1434`)
- `[DEFERRED] OQ-9 - Exact `Find_Nearby_Passable_Cell` destination order for scatter.` (category: `out-of-scope`; reason: separate helper with broad pathfinding behavior; next-step-if-pursued: `/re-investigate Find_Nearby_Passable_Cell scatter destination selection`)

## 8. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| Movement scatter dispatches virtual per-class Scatter; no local universal 8-way draw | `0x00481670`, `0x004B1F43`, `0x0075B891` | mismatch | `bump_crush::scatter_blocker` | Stop using one `RandomRanged(0,7)` equivalent for all blockers; route by blocker class semantics or hold until full Scatter port | `scatter_blocker_unit_null_coord_does_not_consume_eight_way_start_rng` | Do not replace with raw `Next` or `RandomRanged(0,7)`. |
| UnitClass towed gate uses `RandomRanged(1,4) == 1` | `0x00743D2B` | missing/unchecked | future Unit scatter implementation | Consume only in towed, non-threat path | `unit_scatter_towed_gate_uses_randomranged_1_4` | Do not use `1..4` as a direction. |
| UnitClass directional jitter uses `RandomRanged(0,2)-1` | `0x00743DC5`, `0x00743DFF` | missing | future Unit scatter implementation | Apply one draw for real/facing directional scatter | `unit_scatter_directional_jitter_consumes_randomranged_0_2_minus_1` | Do not use eight-way random starts. |
| InfantryClass scatter uses `RandomRanged(0,4)` paths | `0x0051D2BA`, `0x0051D385` | mismatch if modeled by `next_range_u32(8)` | infantry scatter path / `scatter_blocker` for infantry | Use five-way scatter-facing roll where InfantryClass scatter is being modeled | `infantry_scatter_uses_randomranged_0_4_not_0_7` | Do not collapse infantry and vehicle scatter into one helper. |
| Sub-cell center/NW rotation uses `RandomRanged(0,3)` | `0x0048139A`, `INFANTRY_SUBCELL_POSITIONING.md` | none observed | `allocate_sub_cell_with_preference` | Keep `next_range_u32(4)` only after quadrant 0 reaches rotation table | `subcell_center_rotation_uses_randomranged_0_3_only_for_quadrant_zero` | Do not consume RNG for quadrant 2/3/4 fast-path. |
| Idle scatter runs via virtual Scatter every 64-frame phase | `0x004DA530`, `0x004DAE59` | dormant Rust implementation would mismatch if enabled | `scatter::tick_idle_scatter`, `world/mod.rs` | Keep disabled until redesigned around virtual Scatter semantics and `&0x3f == 0x3f` timing | `idle_scatter_disabled_until_virtual_scatter_port` | Do not re-enable current `frame%150` + 8-way RNG. |

## Stale Docs / Follow-up Docs

- `SCATTER_DISPATCH_SYSTEM_DEEP_DIVE.md`: replace broad wording `Compute: 8-direction scan from facing/random` for UnitClass movement/null scatter with `UnitClass directional scatter scans 8 directions after facing/atan2 plus `RandomRanged(0,2)-1`; NullCoord movement scatter uses `Find_Nearby_Passable_Cell` and has no direct `RandomRanged(0,7)` draw in `UnitClass::Scatter`.`
- `SCATTER_DISPATCH_SYSTEM_DEEP_DIVE.md`: replace broad wording `InfantryClass ... wider random range` with `InfantryClass scatter-facing paths consume `Scenario+0x218 RandomRanged(0,4)` at `0x0051D2BA` / `0x0051D385`; do not model this as `RandomRanged(0,7)`.`
- `src/sim/movement/scatter.rs` comments should not claim `IDLE_SCATTER_INTERVAL=150` matches original; verified `FootClass::AI` idle scatter phase is `(g_CurrentFrameCounter & 0x3f) == 0x3f`.

## Sources

- Ghidra decompile: `0x00481670`, `0x00743A50`, `0x0051D0D0`, `0x00481180`, `0x004DA530`
- Ghidra assembly contexts: `0x0048139A`, `0x00743D2B`, `0x00743DC5`, `0x00743DFF`, `0x0051D2BA`, `0x0051D385`, `0x004B1F43`, `0x0075B891`, `0x0074177A`, `0x004DAE59`
- Docs: `UNIT_CLASS_SCATTER_GHIDRA_REPORT.md`, `SCATTER_TRIGGER_POINTS_GHIDRA_REPORT.md`, `SCATTER_DISPATCH_SYSTEM_DEEP_DIVE.md`, `INFANTRY_SUBCELL_POSITIONING.md`, `RNG_SYSTEM_GHIDRA_REPORT.md`
- INI: `ini/rulesmd.ini`, `ini/rules.ini`
- Rust: `src/sim/movement/bump_crush.rs`, `src/sim/movement/scatter.rs`, `src/sim/movement/movement_occupancy.rs`, `src/sim/world/mod.rs`, `src/sim/movement/movement_reservation.rs`, `src/sim/movement/movement_step.rs`
