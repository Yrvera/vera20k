# Spark Collision and Pixel Compositor Investigation Plan

**Date:** 2026-07-18  
**Status:** Awaiting approval  
**Target binary:** active Yuri's Revenge `gamemd.exe`  
**Expected research output:** `docs/research/PARTICLE_SPARK_COLLISION_AND_PIXEL_COMPOSITOR_GHIDRA_REPORT.md`  
**Implementation scope:** none; this plan authorizes research and documentation only

## 1. Goal

Close the two remaining binary-evidence blockers in the approved Spark design without
redoing the already-verified Spark spawn, color, or light work:

1. Recover the exact `ParticleClass` Spark movement and collision contract at
   `0x0062C6E0`, including coordinate frames, units, integer/float conversions,
   gravity ordering, terrain/bridge/building/wall predicates, matrix operations,
   state writes, and deletion timing.
2. Recover the exact single-pixel Spark compositor contract at `0x0062CEC0`, including
   projection, clipping, A-buffer address/value semantics, Z-buffer arithmetic and
   inequality, color selection, brightness scaling, DirectDraw packing, destination
   surface dispatch, and frame ordering.
3. Translate both findings into a Rust-facing implementation handoff that names the
   existing simulation, terrain/bridge/occupancy, A-buffer, and render-depth surfaces.
   The handoff must classify every mechanism difference as DRIFT unless exact
   equivalence is positively proved.

The executed report must answer these load-bearing questions:

- What exact `ParticleClass` fields feed the pre-move position and velocity values,
  and what reference frame and unit does each value use?
- Does the active Spark path apply gravity once, twice, or through two different
  intermediate values? In what order are those values converted, transformed, tested,
  and committed?
- Which coordinate is passed to `CellClass__GetGroundHeight`, which coordinate selects
  the cell, and what signed comparison decides a ground collision?
- What does cell flag `0x100` mean on this active path, and how do bridge deck, building,
  and wall tests change the collision plane or branch?
- What roles do `FUN_00437090`, `FUN_0043A100`, `FUN_006D6AD0`,
  `VXL_GetFacingMatrix`, `FUN_005AFC20`, `FUN_0043A0B0`, `FUN_0043A0D0`, and
  `FUN_005AF4D0` play? Each transform must name source frame, destination frame,
  handedness/axis order, and numeric type.
- On collision, which coordinates, velocities, impact values, flags, and deletion bytes
  are written, and which later code observes them in the same tick?
- What exact world-to-client conversion and viewport offset place the Spark pixel?
- Are clip bounds left/top inclusive and right/bottom exclusive in this caller?
- Is the A-buffer sample treated as a full `ushort`, low byte, signed value, or unsigned
  value? What happens at `0`, `1`, `0x7E`, `0x7F`, and values above `0x7F`?
- What exact signedness and width are used by
  `(zbuffer-origin/bottom - screen_y) - AdjustForZ - 0x32`, and does a pixel draw when
  the candidate is less than, equal to, or greater than the stored Z value?
- Does the Spark path write Z or only sample it?
- How are color index zero and nonzero indices resolved, how are RGB components
  converted/scaled, and how do the runtime DirectDraw loss/shift globals form the final
  16-bit pixel?
- Which concrete `g_PrimarySurface` method implements vtable slot `+0x24`, and what
  clipping/lock/unlock behavior can still suppress the write?
- Can the current Rust GPU depth and fullscreen A-buffer pass reproduce the native
  integer predicate and final packed-pixel result for the full relevant input space?
  If unproved, the report must say DRIFT/UNCHECKED and specify the missing substrate.

## 2. Scope Corrections and Prior Research Inventory

The approved design listed five blockers. Two of them were already resolved by a
high-confidence report written before the design and are explicitly out of scope here:

- First-visible persistent Spark light stage is `0`.
- One-frame radius conversion truncates toward zero after clamp/multiply; for positive
  radii this is floor. The stock and fractional fixtures are already documented.

| Source | Confidence / status | Use in this investigation |
|---|---|---|
| `docs/plans/2026-07-18-spark-particle-system-and-lighting-design.md` | approved design | Supplies the architecture and identifies the two unresolved evidence blockers. Do not implement from it during this task. |
| `docs/research/PARTICLESYSTEMCLASS_GHIDRA_REPORT.md` | HIGH, major functions decompiled | Baseline layouts, dispatch, Spark AI, draw outline, and current address anchors. Its short Spark-AI summary is insufficient for coordinate translation and must not substitute for the requested assembly/dataflow proof. |
| `docs/research/PARTICLE_TIMING_SPARK_RAILGUN_NORMALIZED_GHIDRA_REPORT.md` | High for listed functions | Reuse verified dispatch, lifetime, color progression, and draw gating. Railgun may be used only as a shared pixel-path comparator. |
| `docs/research/SPARK_LIGHT_EFFECT_TICK_ROUNDING_AND_FIRST_VISIBLE_STAGE_RESWARM_20260528.md` | High for scoped mechanism | Authoritative resolution for light-stage ordering and `Math__ftol` rounding. Reference it; do not redo it. |
| `docs/research/BSURFACE_CIRCBUF_ABUFFER_REPORT.md` | HIGH | Reuse circular-buffer layout, viewport-relative addressing, and A-buffer storage. Verify only Spark-specific consumption. |
| `docs/research/ZBUFFER_DEPTH_SYSTEM.md` | binary-backed system map | Reuse global Z-buffer ownership, terrain writes, clear behavior, and known render ordering. Verify the Spark read predicate directly. |
| `docs/research/building-selection-brackets/PRIMARY_SURFACE_ZBUFFER_BRACKET_OWNERSHIP_GHIDRA_REPORT.md` | High for bracket path | Reuse global A/Z buffer identity and tactical clear/write ownership; do not transfer bracket inequalities to Spark without checking `0x0062CEC0`. |
| `docs/research/building-selection-brackets/SURFACE_DRAWLINE_ABUFFER_ZTEST_PIXEL_CONTRACT_GHIDRA_REPORT.md` | High for bracket line raster | Comparator for A/Z semantics only. Spark is a point write through a different surface slot. |
| `docs/research/skirmish-ui/SKIRMISH_PREVIEW_SURFACE_VTABLE_AND_CLIPPING_GHIDRA_REPORT.md` | High for DSurface slots | Reuse `vtable__DSurface @ 0x007E85D4`, `+0x24 -> 0x007BAEB0`, and the point-write lock/unlock contract, then confirm the tactical primary surface uses the same concrete vtable. |
| `ini/rules.ini`, patched by `ini/rulesmd.ini` | stock data authority | Supplies active Spark systems/types and concrete fixture values. |

Research-index scoping found exact evidence for both roots even though the broad
`system=particles` topic map returned no aggregate rows:

- `0x0062C6E0` maps to `PARTICLESYSTEMCLASS_GHIDRA_REPORT.md`.
- `0x0062CEC0` maps to that report, the timing report, and shared engine-service docs.

### Ranges already covered — do not redo

- Spark burst probability, burst count, raw velocity RNG, shared directionless bias,
  facing jitter, forward AI / reverse cleanup, and system lifetime ordering.
- ColorList runtime layout and Spark/Railgun color progression RNG.
- ParticleSystemClass/ParticleClass generic layouts, INI parsers, save/load, and vtables.
- Persistent and one-frame Spark light creation, stage order, radius rounding, expiry,
  and tactical draw ordering.
- Generic CircBuf construction/scrolling and general terrain Z-buffer population.
- Railgun path construction, movement, and laser behavior.

## 3. Function Inventory

The inventory contains 30 functions. `FULL` means assembly plus decompile, callers,
callees, field dataflow, and at least one concrete trace. `MEDIUM` means decompile plus
relevant callsites/assembly around the load-bearing branch. `LIGHT` means identity and
contract confirmation only.

| # | Phase | Address | Current name | Scope reason | Depth | TS-legacy risk |
|---:|---:|---:|---|---|---|---|
| 1 | 1 | `0x0062C6E0` | Spark particle AI | Primary movement/collision root; recover every arithmetic step, call order, state write, and exit. | FULL | Low; stock Spark types reach it |
| 2 | 1 | `0x0062CE40` | `ParticleClass::AI_Dispatch` | Establish pre/post behavior ordering, lifetime decrement, and same-tick deletion consequences. | MEDIUM | Low |
| 3 | 1 | `0x0062E840` | `ParticleSystemClass::AI_Spark` | Confirm owning iteration order and which Spark state enters particle AI; do not redo burst/light internals. | LIGHT | Low |
| 4 | 1 | `0x0062B5E0` | `ParticleClass::Constructor` | Bind initial position, velocity/direction, color, lifetime, and impact-related fields consumed by #1. | MEDIUM | Low |
| 5 | 1 | `0x00437090` | `FUN_00437090` | Called after three `Math__ftol` conversions in both pre/post movement setup; identify coordinate/vector contract. | FULL | Low |
| 6 | 1 | `0x0043A100` | `FUN_0043A100` | Initializes or transforms the matrix state used by the collision path; recover exact semantics. | FULL | Low |
| 7 | 1 | `0x006D6AD0` | `FUN_006D6AD0` | Consumes the candidate world coordinate before VXL matrix selection; identify tactical/terrain frame conversion. | FULL | Medium; check active YR caller role rather than trusting label |
| 8 | 1 | `0x007559B0` | `VXL_GetFacingMatrix` | Supplies a facing matrix to the collision transform; recover facing units and matrix orientation. | FULL | Low; shared live VXL utility |
| 9 | 1 | `0x005AFC20` | `FUN_005AFC20` | Alternative or composed matrix source in the same collision branch; identify selection conditions. | FULL | Medium; semantic role currently unknown |
| 10 | 1 | `0x0043A0B0` | `FUN_0043A0B0` | Mutates matrix state with two signed values; recover rotation/order and units. | FULL | Low |
| 11 | 1 | `0x0043A0D0` | `FUN_0043A0D0` | Applies the literal `1.0` operation between transform stages; identify whether scale/reset/composition. | MEDIUM | Low |
| 12 | 1 | `0x005AF4D0` | `FUN_005AF4D0` | Matrix/vector application used twice; bind input/output layout, precision, and multiplication order. | FULL | Low |
| 13 | 1 | `0x00578080` | `CellClass::GetGroundHeight` | Recover return frame/unit, bridge influence, and the exact comparison performed by #1. | FULL | Low |
| 14 | 1 | `0x00565730` | `MapClass::Get_CellClass_At_Coord` | Bind coordinate-to-cell rounding, invalid-coordinate fallback, and returned cell identity. | MEDIUM | Low |
| 15 | 1 | `0x0047C520` | `Look_up_building_in_cell` | Prove which building presence/flags make Spark collision take the object-contact branch. | FULL | Low |
| 16 | 1 | `0x00480510` | `CellClass::IsWallConnectableInDirection` | Determine why sentinel directions `-1,-1` are passed and what wall predicate that represents. | MEDIUM | Medium; verify this is an active Spark collision test, not a misleading label |
| 17 | 1 | `0x0041C230` | `CoordStruct::Set` | Confirm final float-to-coordinate storage width, argument order, and no hidden clamp. | LIGHT | Low |
| 18 | 2 | `0x0062CEC0` | `ParticleClass::Draw_It` | Primary Spark pixel root; recover all gates, projection, A/Z predicate, color math, packing, and destination write. | FULL | Low; stock Spark and Railgun reach it |
| 19 | 2 | `0x006D2140` | `TacticalClass::CoordsToClient2` | Bind world lepton axes/Z to client pixel axes and all signed rounding/offset rules. | FULL | Low |
| 20 | 2 | `0x006D20E0` | `Tactical::AdjustForZ` | Recover exact Z adjustment formula and input state used by the Spark pixel depth candidate. | FULL | Low |
| 21 | 2 | `0x004114B0` | `CircBuf_GetScanlinePtr` | Confirm Spark caller X/Y arguments, origin subtraction, 16-bit sample, and circular wrap behavior. | MEDIUM | Low; reuse HIGH prior report |
| 22 | 2 | `0x007BD130` | `ZBuffer_scanline_ptr` | Confirm point-address calculation, returned element width, origin, wrap, and default-value interaction. | MEDIUM | Low |
| 23 | 2 | `0x007BAEB0` | DSurface point writer (`vtable +0x24`) | Confirm tactical primary surface dispatch, point bounds, 16-bit write, and lock/unlock failure behavior. | MEDIUM | Low; reuse vtable report then verify tactical instance |
| 24 | 2 | `0x0055AF60` | `FUN_0055AF60` | Identify the measured-performance gate at the start of particle drawing and its comparison domain. | MEDIUM | Low |
| 25 | 2 | `0x005865E0` | `FUN_005865E0` | Identify the conditional shroud/fog coordinate predicate without treating default-off fog as ordinary YR. | MEDIUM | High; TS fog path is normally inactive in stock YR |
| 26 | 2 | `0x005F4CF0` | `ObjectClass::DrawIt` | Confirm how object rendering reaches particle drawing and the one-frame light hook without conflating their vtable slots. | MEDIUM | Low |
| 27 | 2 | `0x006D8DB0` | `Tactical_ObjectRenderingLoop` | Establish object/layer order and the A/Z state present when Spark pixels are attempted. | MEDIUM | Low |
| 28 | 2 | `0x006D3D10` | `TacticalClass_Draw` | Establish full-frame ordering around Z clear, terrain writes, object rendering, persistent lights, and the current primary surface. | MEDIUM | Low; note older docs cite interior `0x006D3F50` for a branch |
| 29 | 2 | `0x006D2B60` | `Tactical_ZBufferDirtyClear` | Confirm the clear point and dirty-rectangle semantics before the Spark read. | LIGHT | Low; reuse prior report |
| 30 | 2 | `0x007BCF50` | `ZBuffer_rect_clear` | Confirm clear value/width and whether untouched pixels can differ from the nominal default. | LIGHT | Low |

### Phase boundaries

- **Phase 1 checkpoint:** Produce a named coordinate-frame diagram and three numeric
  collision traces. If any helper remains semantically ambiguous, stop before writing
  Rust-facing formulas and escalate with assembly/dataflow or a bounded runtime trace.
- **Phase 2 checkpoint:** Produce the exact pixel predicate and two boundary tables
  (A-buffer and Z-buffer). If the primary-surface vtable or stored Z signedness is not
  proved, classify the compositor as unresolved rather than inferring it from bracket
  reports.
- **Synthesis checkpoint:** Reconcile the native contracts with current Rust surfaces.
  This is a handoff only; do not patch Rust or declare pixel parity from hand-computed
  values.

## 4. Detail Checklist

### Phase 1 — Spark movement and collision

- Enumerate every `ParticleClass` and `ParticleTypeClass` field read or written by
  `0x0062C6E0`, with byte offset, width, signedness, semantic role, and evidence.
- Reconstruct arithmetic from assembly, not decompiler temporaries. Record x87 stack
  order, constant values, conversions through `Math__ftol`, and any extended-precision
  lifetime between stores.
- Resolve the apparent double gravity use in the live body. State whether two
  subtractions affect one velocity, two candidate positions, or a collision probe plus
  committed movement.
- Name every coordinate frame and unit before translation: particle-local vector,
  world leptons, cell coordinate, terrain height, bridge height, matrix space, and
  final committed `CoordStruct`.
- Trace the ordinary no-collision branch and every collision exit separately.
- Verify terrain, high-bridge/bridge-deck, building, and wall conditions from the
  bodies/callsites. Do not conflate the active bridge logic with dormant subterranean
  locomotion or low-bridge Tube movement.
- Prove whether a collision marks deletion immediately, stores an impact vector for a
  later effect, changes position before deletion, or combines those actions.
- Confirm color-progression RNG occurs after the movement/collision work on surviving
  and dying branches as applicable; cite branch-specific RNG consumption without
  re-investigating the already-settled formula.
- Cross-check each inferred helper role against at least one additional caller before
  applying a semantic name. Local Ghidra labels are navigation hints only.

Required coordinate fixtures:

1. **Flat-ground impact:** a stock `Spark` particle at a specified lepton coordinate
   and height with explicit X/Y/Z motion and `Rules.Gravity`. Walk every intermediate
   native value through collision decision, committed state, and deletion/lifetime.
2. **Bridge crossing/contact:** use an active YR bridge cell with explicit terrain
   height, bridge flag/deck data, entry coordinate, and motion vector. Show which plane
   is tested and whether the same input above versus below the deck changes the result.
3. **Building/wall contact:** use one occupied cell and one wall-overlay cell with the
   same incoming particle state. Show the exact building lookup/wall predicate and the
   resulting matrix/state/deletion branch.

The expected values for these fixtures must be derived from binary/retail data or a
captured native trace. Hand-authored Rust goldens are not parity evidence.

### Phase 2 — Spark point compositor

- Reconstruct all early gates in order: performance threshold, detail gate, fog/shroud
  conditional, behavior dispatch, client projection, clip rect, A-buffer, Z-buffer,
  color selection, RGB math, and point write.
- Record the exact screen coordinate after `CoordsToClient2` and
  `g_RadarViewportOffsetY`, and distinguish tactical/world pixels from surface/client
  pixels and buffer-relative rows.
- Verify clip arithmetic at left, top, right-1, bottom-1, right, and bottom.
- Prove A-buffer sample width and scaling arithmetic. Include the multiplication width,
  signedness, arithmetic/logical shift behavior, and whether channels clamp before
  DirectDraw packing.
- Prove the Z candidate expression from assembly, including the two `short` loads from
  `g_ZBuffer`, intermediate casts, subtraction order, `0x32` bias, and branch sense.
- Prove whether the stored Z sample is compared signed or unsigned and whether the
  Spark path ever writes it.
- Verify color index zero uses the per-particle start/current color while nonzero uses
  the correct `ColorList` entry and stride. Cover the final valid index and malformed
  out-of-range policy without inventing a safe fallback.
- Record actual runtime DD loss/shift globals for the retail target, but express the
  mechanism data-driven unless the binary proves those values are invariant.
- Confirm `g_PrimarySurface` uses DSurface vtable `0x007E85D4` at the active tactical
  call and slot `+0x24 -> 0x007BAEB0`.
- Establish frame ordering: Z clear, terrain depth writes, object/particle draw,
  persistent Spark light pass, and any later A-buffer application that changes the
  final pixel.

Required pixel fixtures:

- A-buffer values `0`, `1`, `0x7E`, `0x7F`, and one value above `0x7F`, using the same
  RGB and passing Z. Record draw/no-draw and exact packed output.
- Z sample immediately below, equal to, and immediately above the candidate depth,
  using the same A-buffer value and RGB.
- Color index `0` and `1` for stock `WeldingSpark`, including its two start colors and
  first ColorList entry as reached by real constructor state.
- Clip points at all four inclusive/exclusive boundaries.
- A native-derived final pixel capture or executable predicate oracle sufficient to
  validate packed output. A prose calculation alone may inform implementation but may
  not certify pixel parity.

## 5. INI and Retail Data in Scope

YR `rulesmd.ini` patches base `rules.ini`; the merged values are authoritative.

| Key / data | Why it is in scope |
|---|---|
| `BehavesLike=Spark` | Selects both system and particle Spark paths. |
| `HoldsWhat` | Binds the system to the concrete particle type used by fixtures. |
| `MaxEC` | Supplies lifetime observed around particle AI/deletion. |
| `XVelocity`, `YVelocity`, `MinZVelocity`, `ZVelocityRange` | Supply concrete initial motion values for the collision traces. |
| `ColorList`, `ColorSpeed`, `StartColor1`, `StartColor2` | Supply draw-time color fixtures; color timing itself is already verified. |
| `[General]` / Rules gravity field | Supplies the exact active gravity value read by Spark particle AI. |
| Retail map cell/template/overlay/bridge data | Supplies ground, bridge, building, and wall fixtures with active YR semantics. |
| Runtime DirectDraw channel loss/shift globals | Supply exact packed-pixel layout for the active retail display mode. |

Stock anchors include `Spark`, `WeldingSpark`, `FirestormSpark`, and `LargeSpark`.
`Spark` has X/Y maxima `10`, minimum Z `40`, Z range `15`, and `ColorSpeed=.13`;
`WeldingSpark` uses X/Y `16`, the same Z values, two start colors, and a five-entry
ColorList. Re-read merged INI data during execution rather than copying these values as
hardcoded constants.

## 6. Caller and Integration Map

```text
logic tick
  ParticleSystemClass::AI @ 0x0062FD60
    -> AI_Spark @ 0x0062E840
      -> each owned ParticleClass AI in forward order
        -> ParticleClass::AI_Dispatch @ 0x0062CE40
          -> Spark particle AI @ 0x0062C6E0
             movement / terrain-cell probes / bridge-building-wall branch
             matrix transform / coordinate commit / deletion state
             color progression RNG
      -> reverse-order dead-particle cleanup

tactical draw
  TacticalClass_Draw @ 0x006D3D10
    -> Z-buffer clear and terrain depth population
    -> Tactical_ObjectRenderingLoop @ 0x006D8DB0
      -> ObjectClass::DrawIt @ 0x005F4CF0 / object virtual draw
        -> ParticleClass::Draw_It @ 0x0062CEC0
          -> CoordsToClient2
          -> CircBuf_GetScanlinePtr(g_ABuffer)
          -> AdjustForZ + ZBuffer_scanline_ptr(g_ZBuffer)
          -> RGB scale + DD pack
          -> g_PrimarySurface vtable +0x24 -> DSurface point writer
    -> persistent Spark light draw later in the tactical pass
```

The executed report must correct this map if live callsites show a different virtual
slot or an intermediate wrapper. It must distinguish the particle point-draw virtual
from the one-frame Spark light hook at object vtable `+0x114`.

## 7. TS-Legacy and Reachability Risk Register

| Mechanism | Risk | Required verdict |
|---|---|---|
| Spark system/particle AI | Low | Active in stock YR; name stock references and caller path. |
| Ground collision | Low | Active; prove from standard Spark AI body. |
| Bridge/building/wall collision | Medium | Prove each branch is reachable with active YR cell data. Do not infer from TS-era field names. |
| `VXL_GetFacingMatrix` use | Medium | Prove its Spark caller semantics from body and receiver/dataflow; do not trust the label alone. |
| Fog-of-war suppression in `0x0062CEC0` | High | Conditional TS legacy; standard YR default is off. Report it without making it the normal visibility model. |
| A-buffer shroud suppression/modulation | Low | Active; distinguish unexplored shroud from optional fog. |
| Railgun comparison | Low | Active shared point path, but Railgun AI is outside this investigation. |
| Map-editor branch | Medium | Conditional and not standard skirmish gameplay; document only if it changes a scoped predicate. |

Every branch in the final coverage ledger must state `Active in YR: Yes / No /
Conditional` with evidence.

## 8. Current Rust Implementation Surface

This is navigation context, not evidence of native behavior.

| Rust surface | Current relevance / question for handoff |
|---|---|
| `src/sim/particles/mod.rs` | Owns deterministic `ParticleSystemStore` and particle state. Identify missing fields/precision required by native Spark collision. |
| `src/sim/particles/spawn.rs` | Currently rejects Spark/Railgun. Constructor handoff must name exact state needed before this guard can be removed. |
| `src/sim/particles/system_ai.rs` | Currently no-ops Spark/Railgun. Future `spark.rs` boundary must preserve owner iteration and RNG order. |
| `src/sim/particles/fire.rs` | Existing particle/terrain logic is a comparison only; do not reuse unless exact Spark semantics are proved. |
| `src/map/resolved_terrain.rs`, `src/map/terrain.rs` | Hold terrain heights and world/screen conversions. Check whether their frames, units, and rounding can represent the native contract exactly. |
| `src/sim/bridge_state/mod.rs`, `src/sim/map/bridge_topology.rs` | Hold live bridge/deck state. Identify the minimal read-only query Spark AI needs without coupling to render state. |
| `src/sim/occupancy.rs` and entity storage | Candidate building/wall occupancy sources. Prove iteration/selection equivalence before reusing. |
| `src/app_instances/particles.rs` | Current SHP particle builder skips Tier 3 and uses a passthrough sprite path. Spark needs a distinct point path. |
| `src/render/shroud_buffer.rs` | Has an R8 CPU A-buffer and `sample_world`, then a fullscreen GPU multiply. Determine whether native integer modulation can be represented without double-darkening or quantization drift. |
| `src/app_render/draw_passes.rs` | Particles currently draw before the fullscreen shroud multiply and use no depth interaction. Map the native point draw to a proven pass/order. |
| `src/render/batch.rs` and depth attachments | Current scene depth is `Depth32Float`; native Spark samples 16-bit tactical Z. Exact conversion/equivalence is unproved and therefore DRIFT/UNCHECKED until demonstrated. |
| `src/app.rs` | Owns `ShroudBuffer` and app/render state; likely presentation boundary, not a simulation dependency. |

The report must not recommend routing Spark through the current SHP particle atlas,
map/cell lighting, or an approximate white 1x1 sprite merely because those paths exist.

## 9. Deferred and Out-of-Scope Questions

- Spark burst spawning, RNG sequence, color interpolation, persistent lights, and
  one-frame lights are settled sibling scopes.
- Railgun AI, spiral generation, laser lines, and its motion are out of scope. Only
  shared `Draw_It` behavior may be used as a comparator.
- Exact screen-light lookup/composition is not part of the point-pixel investigation;
  the approved design and light report own that behavior.
- Generic DSurface/BSurface behavior beyond the exact methods reached here is out of
  scope.
- Physical presentation under render starvation is deferred unless it changes the
  scoped per-draw predicate.
- Malformed mod data policies may be documented as native crash/undefined behavior,
  but no Rust policy decision or fallback is authorized here.
- No implementation contract, Rust patch, snapshot change, world hash change, render
  pipeline change, build, or golden rebaseline is part of this task. A separate
  implementation-contract step follows successful research.

## 10. Execution Strategy

Use one focused `/re-investigate` session with two static-first phases and checkpoints.
The roots are intertwined through shared coordinates and tactical buffers, so a single
report is preferable to independent reports that could assign incompatible frames or
signedness.

1. **Phase 1 — collision/matrix:** decompile/disassemble #1-#17, build the field ledger,
   resolve helper roles from bodies plus callers, and produce the three numeric traces.
2. **Checkpoint:** reconcile all coordinate frames. If a matrix/helper remains
   ambiguous, use Ghidra p-code dataflow and a bounded read-only runtime trace on the
   active retail binary. Do not guess.
3. **Phase 2 — pixel compositor:** decompile/disassemble #18-#30, prove buffer and
   surface dispatch, and generate A/Z/color/clip boundary tables.
4. **Checkpoint:** compare native integer/packed-pixel semantics with current Rust R8
   A-buffer and float depth surfaces. Mark unproved equivalence as DRIFT/UNCHECKED.
5. **Synthesis:** write the single research report with verified findings separated
   from inference, a complete coverage ledger, negative facts, implementation handoff,
   and executable/native-derived acceptance suggestions.

Estimated scope is medium-large: 30 functions, with 13 FULL, 13 MEDIUM, and 4 LIGHT
targets. Static evidence should be sufficient for most of the work; runtime debugging
is reserved for coordinate/matrix or signed-comparison ambiguity.

## 11. Success Criteria

The executed investigation is complete only when it:

1. Answers every Section 1 question or labels the exact remaining item `UNKNOWN` with
   the failed evidence path and next probe.
2. Provides a byte-offset/width/signedness ledger for every Spark state field read or
   written by the collision and point-draw roots.
3. Provides a coordinate-frame diagram and concrete ground, bridge, and building/wall
   traces with native-derived values.
4. Provides exact A-buffer, Z-buffer, clip, color-source, RGB scaling, DD packing, and
   point-write predicates with assembly/decompile citations.
5. Proves the active primary-surface `+0x24` target and whether Spark writes only color
   or also Z.
6. States active-YR reachability for every scoped branch and explicitly identifies the
   default-off TS fog path.
7. Reconciles findings with prior reports, flagging any stale or contradictory wording
   rather than silently choosing one.
8. Includes a Rust-facing handoff naming exact required state/query/render surfaces and
   treating every unproved mechanism equivalence as DRIFT/UNCHECKED.
9. Names at least one gamemd/retail-derived executable check or capture path for final
   pixel validation. Hand-computed goldens alone are insufficient.
10. Makes no Rust changes and does not begin implementation.

## Sources

- Live Ghidra scoping: callees of `0x0062C6E0` and `0x0062CEC0`; decompile call
  context for both roots; callers/callees around `0x0055AFB0`, `0x006D3D10`, and
  `0x006D8DB0`.
- Research index brief for anchors `0x0062C6E0` and `0x0062CEC0`.
- Prior research and design documents listed in Section 2.
- Merged stock INI authority: `ini/rules.ini` with `ini/rulesmd.ini` patches.
- Current Rust navigation surfaces listed in Section 8.
