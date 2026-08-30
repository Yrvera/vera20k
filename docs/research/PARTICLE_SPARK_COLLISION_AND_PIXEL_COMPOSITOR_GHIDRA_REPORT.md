# Particle Spark Collision and Single-Pixel Compositor — Ghidra Report

**Date:** 2026-07-18  
**Target:** active Yuri's Revenge `gamemd.exe`, image base `0x00400000`, x86 32-bit  
**Primary roots:** Spark particle AI `0x0062C6E0`; `ParticleClass::Draw_It` `0x0062CEC0`  
**Investigation mode:** coverage-map (bounded static slice)  
**Confidence:** HIGH for the static movement, collision, draw-gate, A-buffer, Z-buffer, color, and surface-write mechanisms; PARTIAL for a retail-mode final packed pixel because no running `gamemd.exe` process was available to capture the active DirectDraw masks or a native pixel  
**Implementation scope:** none; this document contains research and a handoff only

**2026-08-27 numeric correction:** active runtime capture in
`PHASE3_CELL_GROUND_HEIGHT_104_DOMAIN_CONSUMER_CENSUS_GHIDRA_REPORT.md`
supersedes the original HIGH-confidence 90/360/340 conclusions in this static
report. Cell ground is 104, Particle's independently owned structural offset is
416, and Spark's ascending commit is ground +396. The collision inequalities,
float integration, RNG, and compositor findings remain independently verified;
the affected numeric text below has been corrected.

## Verdict

The two load-bearing roots are live, Ghidra-backed active-YR mechanisms, and both are now sufficiently recovered for a separate implementation-contract step.

The Spark movement path has a real double-gravity asymmetry. It stores `old_vz - Gravity` back to the particle, but computes the candidate displacement with `old_vz - 2*Gravity`. Ground, structural-bridge, building, and wall contact are tested against the candidate coordinate. Every collision branch commits a coordinate, sets the particle deletion byte, still consumes the color-progression RNG, then reaches same-tick reverse cleanup. The apparent slope-reflected impact vector is stack-local only; it is not written back to particle velocity or another persistent impact field.

The Spark compositor is not a sprite approximation. It projects one world coordinate to one client pixel, clips it, samples a 16-bit A-buffer value, applies a strict integer read-only Z test against a 16-bit tactical Z-buffer, interpolates RGB, optionally scales RGB by A, packs through runtime DirectDraw channel shifts/losses, and calls the primary DSurface point writer. The current Rust R8 fullscreen shroud multiply and `Depth32Float` attachment have not been proved equivalent and are therefore DRIFT/UNCHECKED, not parity.

Two plan identities were corrected:

- `ObjectClass::DrawIt` starts at `0x005F4B10`; `0x005F4CF0` is an interior address immediately before its virtual `+0x114` draw dispatch.
- The dirty Z row-fill is `0x007BCFB0`, not `0x007BCF50`. The latter is a different wrapper/helper.

The report does not certify final-pixel parity. A native retail-mode mask capture and executable pixel oracle remain required.

## Scope and evidence discipline

This investigation reused the already-verified Spark spawn, color-timing, iteration, persistent-light, and one-frame-light reports. It did not re-investigate those systems except where their ordering or fields directly enter the two roots.

Binary claims below come from the live Ghidra MCP project holding `gamemd.exe`. The primary evidence operations were fresh `decompile_function` and `disassemble_function` calls on both roots and their listed helpers, `read_memory` on vtables/code/constants, `get_xrefs_to` on draw roots and helpers, `search_byte_patterns` for virtual `+0x114` dispatches, and `search_strings` plus xrefs for `LaserFence=`. Local labels were treated as navigation hints and were accepted only when the body, callsite, receiver, or RTTI/vtable bytes agreed.

No `gamemd.exe` or `ra2md.exe` process was running during the investigation. Static Ghidra evidence was complete for the scoped mechanism, but runtime display masks, camera multiplier value, and a native final-pixel capture could not be sampled. That is why the final mode is a bounded coverage map rather than an exhaustive runtime slice.

## Active-YR reachability

| Mechanism | Active in YR | Evidence and qualification |
|---|---|---|
| Spark particle AI | Yes | `ParticleClass::AI_Dispatch @ 0x0062CE40` dispatches behavior `3` to `0x0062C6E0`; stock `Spark`, `WeldingSpark`, `FirestormSpark`, and `LargeSpark` use `BehavesLike=Spark`. |
| Ground collision | Yes | Unconditional candidate-ground comparison in `0x0062C6E0`. |
| Structural bridge/deck crossing | Yes | Live Spark body reads `CellClass+0x140 & 0x100`; the same flag is established by the active-YR bridge corpus as the structural/high-bridge body/deck flag. This is not subterranean or Tube movement. |
| Building contact | Yes | Live Spark body calls the new-cell object-list scan at `0x0047C520` and validates `WhatAmI == 6`. |
| Wall-overlay contact | Yes | Live Spark body calls `0x00480510` with `(-1,-1)`; the callee's special path recognizes overlay indices `2`, `0x1A`, and `0xF3`. |
| Laser-fence exception | Conditional | `BuildingTypeClass+0x16BF` is the parsed `LaserFence=` byte and `BuildingClass+0x618` is its frame/connectivity state. Stock INIs contain no active assignment, so the code is live but data-inert in stock. |
| 1x1 undeployer exception | Conditional | Building vtable `+0x80` resolves to a predicate requiring `UndeploysInto` plus a `1x1` foundation. |
| Spark/Railgun point path | Yes | `ParticleClass::Draw_It` selects point drawing for behavior `3` or `4`. |
| Extra-animation/detail suppression | Yes | With `g_ExtraAnimationsEnabled == 0`, behavior `1` and `3` are suppressed; stock Spark behavior `3` is affected. |
| Optional fog predicate | Conditional/default-off | Only when not in map editor, a window exists, and scenario bit `0x1000` is set. `0x005865E0` currently returns zero; standard YR does not use this TS-style fog path by default. |
| A-buffer and Z-buffer consumption | Yes | Direct reads in `0x0062CEC0` before the primary-surface write. |

## Stock data used by the fixtures

Merged YR authority is base `ini/rules.ini` patched by `ini/rulesmd.ini`.

| Item | Stock value |
|---|---|
| `Rules.Gravity` | `6` |
| `Spark` motion | `XVelocity=10`, `YVelocity=10`, `MinZVelocity=40`, `ZVelocityRange=15` |
| `WeldingSpark` motion | `XVelocity=16`, `YVelocity=16`, `MinZVelocity=40`, `ZVelocityRange=15` |
| `WeldingSpark` start colors | `(80,255,255)` and `(255,255,100)` |
| `WeldingSpark` ColorList | `(0,128,255)`, `(255,255,255)`, `(200,200,150)`, `(80,80,80)`, `(0,0,0)` |
| Spark-family ground fixture height | `0` leptons on a flat level-zero cell |
| Terrain level step | `104` leptons |
| Structural bridge plane offset | `416` leptons (`4 * 104`) |

Evidence: direct reads of the stock INIs plus the active-runtime capture and initializer correction in `PHASE3_CELL_GROUND_HEIGHT_104_DOMAIN_CONSUMER_CENSUS_GHIDRA_REPORT.md`; Ghidra initializers `0x0047B220`, `0x0062B4A0`, and `0x0062B540` resolve the 104-lepton level and independently owned 416-lepton bridge offset.

## Field ledger

### `ParticleClass` fields consumed or written

| Offset | Width/type | Signedness/precision | Role | Root access |
|---:|---:|---|---|---|
| `+0x9C` | 4 bytes | signed `i32` | world X in leptons | collision read/write; draw read |
| `+0xA0` | 4 bytes | signed `i32` | world Y in leptons | collision read/write; draw read |
| `+0xA4` | 4 bytes | signed `i32` | world Z/height in leptons | collision read/write; draw read |
| `+0xAC` | 4 bytes | pointer | `ParticleTypeClass*` | both roots |
| `+0xB0..+0xB2` | 3 bytes | unsigned channel bytes | current/per-particle start RGB | color AI and draw index-zero source |
| `+0xB4` | 4 bytes | signed `i32` | ColorList index | color AI and draw |
| `+0xB8` | 8 bytes | `f64` | color interpolation accumulator | color AI and draw |
| `+0x10C` | 4 bytes | `f32` | X velocity/displacement in particle motion convention | collision read only |
| `+0x110` | 4 bytes | `f32` | Y velocity/displacement in particle motion convention | collision read only |
| `+0x114` | 4 bytes | `f32` | persistent Z velocity | collision read/write |
| `+0x128` | 2 bytes | signed `i16` | remaining lifetime | post-dispatch decrement |
| `+0x131` | 1 byte | boolean byte | delete/dead marker | collision/lifetime write; owner cleanup read |

### `ParticleTypeClass` and collision-side fields

| Owner/offset | Width/type | Role |
|---|---:|---|
| `ParticleTypeClass+0x2B0` | `f64` | `ColorSpeed` |
| `ParticleTypeClass+0x2B8` | inline vector vtable pointer | ColorList vector object start |
| `ParticleTypeClass+0x2BC` | pointer | packed `ColorStruct` RGB data |
| `ParticleTypeClass+0x2C0` | `i32` | ColorList capacity |
| `ParticleTypeClass+0x2C4` | byte | vector initialized flag |
| `ParticleTypeClass+0x2C5` | byte | vector owns-buffer flag |
| `ParticleTypeClass+0x2C8` | signed `i32` | active ColorList count |
| `ParticleTypeClass+0x2CC` | signed `i32` | vector growth step |
| `ParticleTypeClass+0x2D4..+0x2D6` | 3 bytes | `StartColor1` RGB |
| `ParticleTypeClass+0x2D7..+0x2D9` | 3 bytes | `StartColor2` RGB |
| `ParticleTypeClass+0x2DC` | signed `i16` | `MaxDC` |
| `ParticleTypeClass+0x2E0` | signed integer field | `MaxEC` consumed by construction |
| `ParticleTypeClass+0x2E8` | signed `i32` | `Damage`; nonzero overrides the draw performance gate |
| `ParticleTypeClass+0x314` | signed `i32` | behavior enum; Spark is `3`, Railgun is `4` |
| `BuildingTypeClass+0x16BF` | byte | parsed `LaserFence=` flag |
| `BuildingClass+0x618` | signed integer state | laser-fence frame/connectivity state; `>= 8` suppresses contact |
| `CellClass+0x140` | bitfield | bit `0x100` is the structural/high-bridge body/deck flag on this active path |
| `CellClass+0x44` | overlay index field | `2`, `0x1A`, or `0xF3` satisfy the sentinel wall query |

Evidence: root disassembly/decompilation, `ParticleClass` constructor `0x0062B5E0`, ParticleType construction/parser reads, `LaserFence=` string at `0x0081AA30` and its `BuildingTypeClass::ReadINI` xref near `0x00460AA9`, and the building vtable/RTTI proof described below.

## Coordinate and numeric-frame diagram

| Stage | Input frame/unit | Operation and conversion | Output frame/unit |
|---|---|---|---|
| Persistent motion | particle Z velocity, `f32` leptons/tick | store `old_vz - Gravity` | particle `+0x114`, `f32` leptons/tick |
| Collision probe vector | particle motion convention, `f32` | `(vx, vy, stored_vz - Gravity)` | probe displacement `(vx, vy, old_vz - 2g)` |
| Old coordinate | world coordinate, signed `i32` leptons | convert each component to `f32`, then `Math__ftol`, then copy three dwords | old `CoordStruct`, signed `i32` world leptons |
| Candidate coordinate | old world coordinate plus probe, `f32` | `0x0043A100` adds the vector; `Math__ftol` truncates to signed integer components | candidate `CoordStruct`, signed `i32` world leptons |
| Cell selection | candidate X/Y, signed world leptons | `(v + ((v >> 31) & 255)) >> 8` | signed cell coordinate, truncation toward zero by 256 |
| Terrain query | candidate cell | cell terrain level/slope lookup | ground height, signed world-Z leptons; slope byte |
| Bridge query | old and candidate cells plus Z | cell flag `0x100`, compare with `ground + 416` | collision kind and snapped world-Z |
| Slope-local impact probe | `(vx, -vy, old_vz - 2g)`, `f32` | inverse slope matrix, negate local Z, forward slope matrix, negate final Y | stack-local reflected vector, `f32`; not persistent |
| Commit | selected candidate/snap/clamp coordinate | three signed dword stores to particle coordinate | `ParticleClass+0x9C..+0xA4` |
| Draw projection | signed world leptons | isometric integer projection plus Z adjustment and tactical offsets | signed client/surface pixel coordinate |

This is a world-lepton-to-cell and world-lepton-to-client pipeline. Cell axes are not isometric screen directions. The slope matrix index is a terrain slope byte, not the particle's facing.

## Spark movement and collision contract

### Tick order and the double-gravity result

At `0x0062C6E0`, the Z path performs two distinct subtractions:

1. `Particle+0x114` is overwritten with `old_vz - Rules.Gravity`.
2. The local displacement used to build the candidate coordinate subtracts gravity again, producing `old_vz - 2*Rules.Gravity`.

X and Y use the unchanged `f32` values at `+0x10C` and `+0x110`. The old signed coordinate is converted to `f32`; the displacement is added in `f32`; each candidate component then passes through `Math__ftol` before the new `CoordStruct` is used for cell and terrain queries. The committed Z velocity therefore differs from the Z displacement used for that same tick.

Evidence: assembly ranges `0x0062C705..0x0062C71C` and `0x0062C736..0x0062C76A`, followed by the two three-component `Math__ftol`/`0x00437090` construction sequences in the fresh root disassembly.

The integer `CoordStruct` does not replace the retained candidate `f32` locals for every later predicate. Fresh live Ghidra MCP `disassemble_function(address="0x0062C6E0", program="gamemd.exe")` shows the candidate integer X/Y selecting the cell and terrain query, and the candidate integer coordinate participating in structural-bridge comparisons. However, `0x0062C85B..0x0062C879` loads the integer ground height with `FILD` and compares it directly against the retained candidate-Z `f32`, including the `candidate_z - 150.0` building band. The collision-resolution block beginning at `0x0062C8D2` likewise compares and selects the candidate-Z `f32`; only the final committed coordinate crosses `Math__ftol` at `0x0062CA3B`. An implementation must therefore retain both forms: integer candidates for cell/bridge work, and the raw candidate `f32` for ground, building/wall, near-ground clamp, and final coordinate selection.

### Process x87 control mode and exact store boundaries

The active Spark arithmetic is not governed by the static initializer of the
saved-control-word slot alone. The startup path establishes the control mode in
two verified stages:

1. CRT entry `0x007CD80F` calls `0x007CBDAF`. Its initialized
   function pointer at `0x0087BEB8` resolves to `0x007C8F46`, which
   calls `0x007CEAAF`. That helper calls the control-word routine with
   abstract value `0x10000` and mask `0x30000`. The mapper at
   `0x007CC01C` converts that request to hardware precision-control bits
   `0x0200`: 53-bit/double precision.
2. `WinMain` pushes `0x300, 0x300` and calls `0x007CBF49` at
   `0x006BBFC1`. The same mapper converts the abstract rounding-control
   value `0x300` to hardware bits `0x0C00`: truncate toward zero,
   while preserving the already-selected precision. `WinMain` then calls
   `0x007C5EE4` at `0x006BBFC9` to store the live control word in
   `0x00822D80`. The resulting mode is `0x0E7F`.

`Math__ftol @ 0x007C5F00` reads the current control word, compares it
with the saved word, loads the saved word only when different, and performs
`FISTP qword`. It does not restore a previous mode. Spark therefore runs
under one process-established 53-bit/truncate mode; the conversion helper is not
a local round-mode bracket.

The movement root contains load-bearing memory-rounding points:

- `0x0062C705..0x0062C71C` computes `old_vz - Gravity` and
  immediately stores it to the persistent `f32` field.
- The stored Z value is loaded, Gravity is subtracted again, and the probe is
  explicitly stored as `f32` at `0x0062C75E..0x0062C76A`.
- Each signed coordinate is loaded through `FILD` and stored to a local
  `f32` before the old-coordinate and candidate `Math__ftol` calls.
- `0x0043A100` adds the three stored `f32` components in place,
  producing another explicit `f32` boundary before candidate conversion.

The component operations are not bundled one coordinate at a time. Live Ghidra
MCP `disassemble_function(address="0x0062C6E0", program="gamemd.exe")` fixes the
order as: persistent Z store; X/Y/Z coordinate `FILD`-to-`f32` stores; probe-Z
store; old-coordinate `Math__ftol` calls in Z/Y/X order; vector addition in
X/Y/Z order; candidate `Math__ftol` calls in Z/Y/X order. The final selected
coordinate is likewise converted Z/Y/X at `0x0062CA3B..0x0062CA53` after any
collision delete-byte write and before the coordinate setter.

Color progression likewise has an exact unreassociated sequence:

`((rng * (1 / 2147483646)) * 0.05) + ColorSpeed + old_accumulator`.

The double at `0x007E3570` decodes to
`4.6566128774142013e-10`, exactly `1 / 2147483646`; the
second multiplier at `0x007E8AE8` is double `0.05`. The root
stores the resulting accumulator as `f64` before its comparison with
double `1.0`. These operation and store boundaries are required inputs to
any deterministic Rust compatibility design.

### Helper roles, frames, and precision

| Address | Verified role in this path | Frame/precision consequence |
|---:|---|---|
| `0x00437090` | copies three dwords into a coordinate/vector object | no clamp or hidden conversion |
| `0x0043A100` | adds a three-component `f32` vector in place | old world coordinate in `f32` plus probe displacement |
| `0x006D6AD0` | maps the candidate coordinate to a cell and returns `CellClass+0x11C` slope byte | terrain slope index, not facing |
| `0x007559B0` | copies 12 floats from `g_VXL_FacingMatrices + slope*0x30` | a 3x4 orthonormal slope matrix; entry zero is identity |
| `0x005AFC20` | forms the inverse of an orthonormal 3x4 matrix | transposes the 3x3 part and computes negative-dot translation |
| `0x0043A0B0` | writes three dwords into a vector | constructs the axis-adjusted impact probe |
| `0x0043A0D0` | scales a three-component vector by `f32 1.0` | numerical no-op, but preserves native operation order |
| `0x005AF4D0` | multiplies a vector by only the matrix 3x3 | ignores translation; `f32` result |
| `0x0041C230` | generic three-dword `CoordStruct::Set` | verified helper identity, but no xref from the Spark root; final Spark commit is direct |

The exact transient transform is:

`(vx, -vy, old_vz - 2g)` → inverse slope 3x3 → negate slope-local Z → forward slope 3x3 → negate final Y.

Fresh helper-level verification fixes the scalar operation order that an implementation must preserve. Live Ghidra MCP `disassemble_function(address="0x005AF4D0", program="gamemd.exe")` shows that each output component stays on the x87 stack through two additions and is stored to `f32` only once at the end of that component. In matrix-array order, the three dot products are evaluated as `(m01*v1 + m02*v2) + m00*v0`, `(m11*v1 + m10*v0) + m12*v2`, and `(m21*v1 + m20*v0) + m22*v2`; no intermediate product or partial sum is rounded through an `f32` store. Live Ghidra MCP `decompile_function(address="0x005AFC20", program="gamemd.exe")` plus `disassemble_function(address="0x005AFC20", program="gamemd.exe")` confirms that the inverse helper copies the transposed 3x3 entries as raw `f32` dwords and computes each negative-dot translation with one final `f32` store. The Spark path's subsequent 3x3 multiply ignores those translation entries.

The root fixes the surrounding helper-call sequence as well. Live Ghidra MCP `disassemble_function(address="0x0062C6E0", program="gamemd.exe")` shows inverse-matrix construction at `0x0062C985`, the first matrix-vector call at `0x0062C9C8`, copying its three stored-`f32` outputs, scalar multiplication of that local vector by `1.0f` at `0x0062C9EA`, local-Z negation at `0x0062C9EF..0x0062C9FD`, the forward matrix-vector call at `0x0062CA07`, and final-Y negation at `0x0062CA20..0x0062CA26`. Multiplying the original axis probe before the inverse call is not the native helper sequence, even though multiplication by one often leaves finite output bits unchanged.

The original and inverse transforms are `f32`. The operation defines the axis-sign conversion used here; no broader claim about every VXL matrix convention is needed. The resulting vector remains in root stack locals. Searches of the collision exits and direct assembly show no write to `+0x10C`, `+0x110`, `+0x114`, or another persistent impact vector after this transform. The only persistent collision state is the chosen coordinate plus deletion byte.

### Cell and ground semantics

Both `0x00578080` and `0x00565730` reduce signed world X/Y to cell axes by truncation toward zero at 256 leptons per cell. Negative coordinates therefore do not arithmetic-floor. Invalid coordinates return the dummy cell at `0x00ABDC50` and record the packed requested cell.

Ground height is based on signed cell terrain level (`CellClass+0x11B`) times the 104-lepton level height plus slope-table contribution from `CellClass+0x11C`. The candidate coordinate, not the old coordinate, supplies the primary cell and ground query. Old and new cells are both consulted for the structural-bridge crossing test.

### Collision decision table

Let `G` be candidate-cell ground height and `P = G + 416` be the structural bridge plane.

| Condition | Exact predicate | Resulting Z | Delete byte |
|---|---|---:|---|
| Descending bridge crossing | either old/new cell has bit `0x100`, `newZ < P`, `oldZ >= P` | `P` | `1` |
| Ascending bridge crossing | either old/new cell has bit `0x100`, `newZ >= P`, `oldZ < P` | `P - 20` | `1` |
| Below ground, near surface | `newZ < G` and `G - 100 < newZ` | `G` | `1` |
| Below ground, deep | `newZ < G` and `newZ <= G - 100` | unchanged below-ground value | `1` |
| Building/wall contact band | `G <= newZ` and `newZ - 150 < G`, plus accepted building or wall | `G` | `1` |
| No collision | none of the above | candidate Z | unchanged here |

The building/wall height band is exactly `[G, G + 150)`. `newZ == G + 150` is excluded. The generic near-ground clamp is strict: `newZ == G - 100` is not clamped.

For bridges, equality belongs to the high side: `newZ == P` is not a descending crossing, while `oldZ == P` can satisfy the descending old-side predicate. An ascending crossing writes `P - 20`, deliberately placing the particle below the plane before deletion.

### Building and wall selection

`0x0047C520` scans the candidate cell's object list at `CellClass+0xE4`, following each object's `+0x30` link. With `g_GameActive` set, it returns the first object whose `WhatAmI` virtual (`+0x2C`) is `6`, establishing a building candidate and list-order dependence.

A building candidate is ignored when either condition is true:

- its type has `LaserFence=` at `BuildingTypeClass+0x16BF` and `BuildingClass+0x618 >= 8`; or
- its BuildingClass vtable `+0x80` predicate returns true.

The second virtual was independently resolved from RTTI: vtable `0x007E3EBC`, complete-object locator `0x007FC360`, type descriptor `0x00818D60` named `.?AVBuildingClass@@`, slot target `0x00457620`, wrapper to `0x00465D40`. It returns true only when `BuildingTypeClass+0x408` (`UndeploysInto`) is non-null and the foundation at `+0xEF0` is exactly `1x1`.

If no accepted building exists, `0x00480510` receives `(-1,-1)`. In this sentinel form it does not use a direction; it returns true exactly for overlay indices `2`, `0x1A`, or `0xF3`.

### Commit, RNG, lifetime, and cleanup ordering

Every branch reaches coordinate commit before color progression. Collision branches set `Particle+0x131 = 1`, but do not skip the already-verified color RNG call. `ParticleClass::AI_Dispatch @ 0x0062CE40` then decrements the signed `i16` lifetime at `+0x128`; only a post-decrement value equal to zero sets the deletion byte through the lifetime path. A starting lifetime of zero becomes `-1` and does not delete from lifetime alone.

The collision-path write order is narrower than the summary above: the delete byte is stored first, then the final coordinate is converted and committed. Live Ghidra MCP `disassemble_function(address="0x0062C6E0", program="gamemd.exe")` shows `MOV byte ptr [EBP+0x131],1` at `0x0062CA34`, the final component `Math__ftol` calls at `0x0062CA3B..0x0062CA53`, the coordinate virtual call at `0x0062CA6C`, and only then `Random__RandomRanged` at `0x0062CA86`. Live Ghidra MCP `disassemble_function(address="0x005F6940", program="gamemd.exe")` confirms that the bound coordinate setter performs only the three dword stores at `Particle+0x9C/+0xA0/+0xA4`. Therefore the exact persistent sequence for a collision is already-updated Z velocity, delete byte, coordinate, color RNG/color state, then lifetime.

`ParticleSystemClass::AI_Spark @ 0x0062E840` iterates particle AI forward, then scans dead particles in reverse and invokes their deletion/uninit virtual in the same owning system tick. Thus a colliding particle commits its collision coordinate, consumes one color-progression RNG step, decrements lifetime, and is then eligible for cleanup that tick.

## Required collision traces

These are binary/INI-derived static traces. They document the mechanism but are not executable parity certification.

### Trace 1 — flat-ground impact

Inputs: flat cell, `G=0`, slope index `0`, no bridge/building/wall; world coordinate `(2560,2560,10)` leptons; velocity `(0,0,0)` as `f32`; `Gravity=6`; lifetime `2`.

| Stage | Native value |
|---|---|
| Stored Z velocity | `0 - 6 = -6` |
| Candidate Z displacement | `-6 - 6 = -12` |
| Candidate coordinate | `(2560,2560,-2)` after `f32` addition and `Math__ftol` |
| Ground test | `-2 < 0`; collision |
| Clamp test | `-100 < -2`; clamp to `0` |
| Slope transform | identity slope: transient `(0,0,-12)` reflects to `(0,0,12)`; not stored |
| Persistent result before cleanup | coordinate `(2560,2560,0)`, Z velocity `-6`, delete byte `1` |
| Post-collision order | one color RNG step; lifetime `2 → 1`; reverse cleanup same system tick |

### Trace 2 — structural bridge crossings

Inputs: flat ground `G=0`, old or new cell has structural bit `0x100`, so `P=416`.

| Variant | Old state and velocity | Stored/probe Z | Candidate | Predicate/result |
|---|---|---|---:|---|
| Descending | `oldZ=426`, `old_vz=0` | stored `-6`, probe `-12` | `414` | `414 < 416` and `426 >= 416`; snap to `416`, delete |
| Ascending | `oldZ=406`, `old_vz=30` | stored `24`, probe `18` | `424` | `424 >= 416` and `406 < 416`; force to `396`, delete |
| Equality boundary | choose probe yielding `newZ=416` from above | data-dependent | `416` | descending predicate false because it requires `newZ < 416` |

Both collision variants then commit, consume the color RNG step, decrement lifetime, and enter same-tick cleanup eligibility.

### Trace 3 — building and wall contact

Inputs: flat ground `G=0`, slope index `0`, old Z `100`, old Z velocity `18`, `Gravity=6`. Stored Z velocity is `12`; probe displacement is `6`; candidate Z is `106`, inside `[0,150)`.

| Candidate-cell content | Exact lookup result | Outcome |
|---|---|---|
| Ordinary building | first linked object with `WhatAmI == 6`; neither exception true | contact; clamp Z to `0`; delete |
| Wall overlay | no accepted building; overlay `2`, `0x1A`, or `0xF3` | contact; clamp Z to `0`; delete |
| Active laser-fence exception | type `LaserFence` byte nonzero and frame/connectivity `>= 8` | building suppressed; with no wall/other collision, commit Z `106` and survive this branch |
| 1x1 undeployer exception | `UndeploysInto` non-null and foundation `1x1` | building suppressed; with no wall/other collision, commit Z `106` and survive this branch |

## Single-pixel compositor contract

### Correct caller and vtable chain

The plan's generic chain is correct only after one slot correction:

1. `TacticalClass_Draw @ 0x006D3D10` enters `Tactical_ObjectRenderingLoop @ 0x006D8DB0`.
2. At `0x006D916C`, the renderer calls object virtual `+0x104`. For `ParticleClass`, vtable `0x007EF954` has `+0x104 -> 0x005F4B10`, `ObjectClass::DrawIt`.
3. `ObjectClass::DrawIt` performs outer visibility/rectangle work and at `0x005F4CFD` calls virtual `+0x114`.
4. `ParticleClass` vtable `+0x114`, stored at `0x007EFA68`, points to `0x0062CEC0`.

The nearby renderer calls at `0x006D9153` and `0x006D9789` are virtual `+0x110`. For `ParticleClass`, `+0x110` points to a `RET 8` no-op stub at `0x00426450`; they are not the point-draw dispatch.

RTTI/read-memory proof: `ParticleClass` constructor `0x0062B5E0` stores vtable `0x007EF954`; vtable-minus-four points through COL `0x00807BE8` to type descriptor `0x008366E8`, named `.?AVParticleClass@@`; raw vtable bytes at `0x007EFA68` contain `0x0062CEC0`.

`ParticleSystemClass` separately stores vtable `0x007EFB9C`; its own `+0x114` points to `0x0062E280`, the one-frame light hook. Same numeric slot, different receiver and vtable.

### Early gates in exact order

| Order | Gate | Exact result |
|---:|---|---|
| 1 | measured-performance gate | `0x0055AF60` returns a hysteretic threshold; draw continues when the unsigned measured/global comparison passes, or when `ParticleTypeClass+0x2E8 Damage != 0` overrides it |
| 2 | detail/extra-animation gate | when `g_ExtraAnimationsEnabled == 0`, behaviors `1` and `3` are suppressed; Spark behavior `3` stops here, Railgun behavior `4` does not |
| 3 | optional fog gate | only outside map editor, with non-null window and scenario bit `0x1000`; `0x005865E0` returns zero in this binary, so it does not suppress |
| 4 | behavior branch | behaviors `3` and `4` take the point path; all others take the SHP path |
| 5 | projection and caller clip | failure/outside stops |
| 6 | A sample | zero stops |
| 7 | Z test | only strict pass continues |
| 8 | color/interpolation/A scaling/DD packing | produces packed destination value |
| 9 | DSurface point writer | a null/bounds/lock failure can still prevent the physical write |

`0x0055AF60` uses a latch: with latch clear it compares the measured value to base `0x00829FF4`, sets the latch on a low result, and returns base plus a delta; while latched it returns that sum until the measured value reaches it, then clears and returns the base. The root's assembly comparison is unsigned.

### World-to-client projection

`TacticalClass::CoordsToClient2 @ 0x006D2140` consumes signed `i32` world leptons. Before tactical viewport subtraction:

- planar X is the signed, truncation-toward-zero result of the 60/2 X and -60/2 Y terms, then division by 256;
- planar Y evaluates the signed, truncation-toward-zero `(x*30)/2` and `(y*30)/2` terms separately, adds them, then divides by 256;
- Z adjustment is `Math__ftol(z * g_AdjustForZ_Multiplier + (z >= 728 ? 1 : 0) + 0.5)`.

Intermediate integer additions/multiplications retain native 32-bit wrap behavior. Output X subtracts `TacticalClass+0xB0`; output Y subtracts the Z adjustment and `TacticalClass+0xB4`. The Spark root then adds `g_RadarViewportOffsetY` to Y only. It ignores the helper's boolean and applies its own rectangle clip.

`g_AdjustForZ_Multiplier` is initialized by the display/camera path around `0x006D1BA8` from `60.0 / global * cosine lookup`. Its mechanism is static-verified; its active standard-session numeric value was not captured because no process was running.

Projection fixture with tactical offsets and Z all zero:

| World coordinate | Client coordinate |
|---|---|
| `(256,0,0)` | `(30,15)` |
| `(-1,0,0)` | `(0,0)` because signed division truncates toward zero |

### Clip boundaries

The root uses `x >= left`, `x < left + width`, `y >= top`, and `y < top + height`.

| Point | Result |
|---|---|
| `(left, top)` | inside |
| `(right-1, bottom-1)` | inside |
| `x = right` | outside |
| `y = bottom` | outside |
| `x = left-1` | outside |
| `y = top-1` | outside |

### A-buffer address, width, and modulation

The root calls `CircBuf_GetScanlinePtr @ 0x004114B0` with X and row `screenY - *(i32 *)(g_ABuffer+4)`. The helper locks through the surface virtual, uses the circular backing range, performs one wrap when required, and unlocks. The root loads `word ptr` and zero-extends the complete `ushort`; it does not use only the low byte and does not sign-extend.

Rules:

- `A == 0`: no draw.
- `1 <= A < 127`: each signed interpolated `i32` channel is multiplied by unsigned A with a 32-bit `IMUL`, then arithmetic-shifted right by 7.
- `A >= 127`: RGB is left unchanged. There is no clamp and no special handling for values above `0x7FFF`.

Using passing Z and RGB `(80,255,255)`, with `P(r,g,b)` denoting runtime DD packing:

| A sample | RGB after A stage | Draw/output |
|---:|---|---|
| `0` | n/a | no draw |
| `1` | `(0,1,1)` | `P(0,1,1)` |
| `0x7E` / `126` | `(78,251,251)` | `P(78,251,251)` |
| `0x7F` / `127` | `(80,255,255)` | `P(80,255,255)` |
| `0x80` / `128` | `(80,255,255)` | `P(80,255,255)` |
| `0xFFFF` | `(80,255,255)` | `P(80,255,255)` |

The `126 → 127` discontinuity is native. A current Rust R8 sample can represent only `0..255`, not the complete native `ushort` domain.

### Z-buffer address and strict predicate

The scanline row is `screenY - *(i32 *)(g_ZBuffer+4)`. The candidate is:

`base16 = u16(i16[g_ZBuffer+0x24] + i16[g_ZBuffer+0x04] - screenY)`

`candidate = i32(base16) - AdjustForZ(worldZ) - 0x32`

The addition and screen-Y subtraction wrap to the low 16 bits before zero-extension. The stored Z sample is loaded as a word and zero-extended. The draw predicate is a strict signed 32-bit comparison:

`candidate < i32(stored_u16)`

Equality does not draw. The Spark point path never writes Z.

| Stored sample relative to candidate | Result |
|---|---|
| `candidate - 1` | no draw |
| `candidate` | no draw |
| `candidate + 1` | draw |
| any stored `u16`, candidate negative | draw |
| any stored `u16`, candidate greater than `65535` | no draw |

The actual dirty-row clear at `0x007BCFB0` writes `0xFFFF` words (and paired `0xFFFFFFFF` dwords) across circular backing and wraps. `Tactical_ZBufferDirtyClear @ 0x006D2B60` reaches this fill for dirty rectangles. Untouched values can retain prior populated depth; the point path merely samples what is present.

### Color source and interpolation

For ColorList index zero, current RGB is the three per-particle bytes at `+0xB0..+0xB2`. For a nonzero index, current RGB is `ColorList[index]` at `*(type+0x2BC) + index*3`. In both cases next RGB is `ColorList[index+1]`.

Per channel, the root computes `Math__ftol((1.0 - accumulator) * current + accumulator * next)`, using the `f64` accumulator at `+0xB8` and unsigned byte channels. There is no channel clamp before packing. Normal timing keeps the index at most `count-2`; malformed out-of-range indices are unchecked native memory access, not a safe fallback.

Live Ghidra MCP `disassemble_function(address="0x0062CEC0", program="gamemd.exe")` fixes the cross-channel stack lifetime and add order. `0x0062D164..0x0062D16D` computes `1.0 - accumulator` once and retains that x87 value through all three channels. Each channel then evaluates `next * accumulator`, evaluates `current * retained_one_minus`, and uses `FADDP` to add the current term into the next term before `Math__ftol` (`0x0062D196..0x0062D1AC`, repeated at `0x0062D1CB..0x0062D1E1` and `0x0062D1FF..0x0062D215`). Recomputing `1.0 - accumulator` per channel or reversing the named add operands is not the native operation trace.

For stock `WeldingSpark`:

| Index | Current source | Next source |
|---:|---|---|
| `0` | constructor-selected start `(80,255,255)` or `(255,255,100)` | `ColorList[1] = (255,255,255)` |
| `1` | `ColorList[1] = (255,255,255)` | `ColorList[2] = (200,200,150)` |

The first transition does not target `ColorList[0]`; a randomized start at index zero transitions to list entry one.

### DirectDraw packing and destination write

For each signed `i32` channel, the root arithmetic-shifts by the runtime Loss global, shifts left by the runtime Shift global, masks to 16 bits, then ORs the three channels:

`P(R,G,B) = ((R >> RLoss) << RShift & 0xFFFF) | ((G >> GLoss) << GShift & 0xFFFF) | ((B >> BLoss) << BShift & 0xFFFF)`

The globals are at `RShift 0x008A0DD0`, `RLoss 0x008A0DD4`, `BShift 0x008A0DD8`, `BLoss 0x008A0DDC`, `GShift 0x008A0DE0`, and `GLoss 0x008A0DE4`. DSurface construction around `0x004BA770` obtains DirectDraw masks and derives shifts/losses, allowing 555/565-like runtime layouts. Static evidence does not prove one invariant retail numeric layout, so this report deliberately does not substitute an assumed RGB565 constant for the missing runtime capture.

The active class identity is independently proved: DSurface vtable `0x007E85D4`, vtable-minus-four COL `0x00800260`, type descriptor `0x008205D8` named `.?AVDSurface@@`, and `+0x24` raw target `0x007BAEB0`. The WinMain construction chain creates the primary/backbuffer DSurface wrappers and assigns the primary wrapper global.

`0x007BAEB0` asks the surface virtual `+0x5C` for the target pixel pointer. A null result returns false with no write. It queries bytes per pixel via `+0x70`; at two bytes per pixel it writes the packed 16-bit value, otherwise it writes only the low byte; it then unlocks via `+0x60`. The Spark root ignores the method's boolean result. There is no Z-buffer update.

## Visual/UI composition ledger

| Order | Native stage | Input consumed | Output/state effect | Spark consequence |
|---:|---|---|---|---|
| 1 | Tactical dirty Z clear (`0x006D4471` callsite) | dirty rectangles/circular Z backing | relevant words become `0xFFFF` | establishes far/default depth |
| 2 | terrain, bridge, shroud, and object-preparation layers | tactical cells and render lists | populate Z and A state | Spark later samples these exact buffers |
| 3 | `Tactical_ObjectRenderingLoop` (`0x006D465F`) | active objects/layer order | calls `ObjectClass::DrawIt`; Particle point draw may write one primary-surface pixel | A and Z tests occur immediately before the write |
| 4 | persistent-light drawing (`0x006D4664`) | active light collection | later light composition | occurs after Particle point attempt |
| 5 | later tactical/UI presentation | primary/back surfaces | present/composite | no scoped native full-screen A multiply is applied to the already A-modulated point |

The native point reads A before writing color. Therefore a Rust path that first premodulates Spark RGB and later applies the current fullscreen shroud multiply would modulate twice.

### Asset and presentation-role matrix

| Role | Native source | Used by Spark point path? | Rust implication |
|---|---|---:|---|
| Particle bitmap/SHP | particle type image virtual | No for behavior `3`; only non-point behaviors use the SHP branch | do not route Spark through the existing SHP atlas |
| Start RGB | `ParticleClass+0xB0..+0xB2`, seeded from INI start colors | Yes | per-particle state, not a white texture approximation |
| Color progression | packed `ColorList` RGB data | Yes | preserve byte entries and native indexing |
| Shroud/visibility modulation | tactical A-buffer `ushort` | Yes | point-time integer sample, not merely later fullscreen multiplication |
| Occlusion | tactical Z-buffer `ushort` | Yes, read-only | needs native-compatible integer predicate/substrate |
| Destination | active primary DSurface | Yes | one packed point write; no sprite quad required by native mechanism |

## Adversarial and boundary cases

| Case | Verified outcome |
|---|---|
| Lifetime starts at zero | signed `i16` decrement produces `-1`; lifetime equality does not delete |
| Negative world X/Y | cell and projection division truncate toward zero, not floor |
| Bridge candidate exactly on plane | not a descending crossing; descending requires strict `<` |
| Building/wall candidate exactly `G+150` | excluded from contact band |
| Below-ground candidate exactly `G-100` | not clamped; strict clamp predicate is false |
| A-buffer `126` versus `127` | discontinuous from scaled `(78,251,251)` to unscaled `(80,255,255)` for the fixture |
| A-buffer above `0x7FFF` | remains unsigned and, because `>=127`, leaves RGB unchanged |
| Wrapped Z base | origin/bottom sum minus screen Y wraps to `u16` before subtracting Z adjustment and 50 |
| Negative Z candidate | passes against every zero-extended stored `u16` |
| Color index out of range | unchecked native access; no verified fallback |
| DSurface lock/bounds failure | returns false/no pixel write; caller ignores result |
| Non-two-byte surface | point writer writes only packed low byte |
| Collision and color RNG | deletion marker does not bypass the same-tick color RNG step |
| Slope reflection result | stack-local only; no persistent bounce/impact velocity |

## Cold spot-checks and zero-add pass

Three independent cold checks were performed after the main roots had been reconstructed:

1. **Z clear identity:** fresh decompile/disassembly showed that `0x007BCF50` was not the planned dirty rect fill. Caller tracing from `0x006D2B60` identified `0x007BCFB0`, whose body writes `0xFFFF`/`0xFFFFFFFF` and wraps the circular backing.
2. **DSurface writer:** RTTI/COL and raw vtable bytes independently bound DSurface `+0x24` to `0x007BAEB0`; its body confirmed pointer acquisition, two-byte/one-byte writes, unlock, and failure behavior.
3. **Particle draw dispatch:** raw `ParticleClass` vtable bytes, byte-pattern search for `CALL [vtable+0x114]`, and fresh `Tactical_ObjectRenderingLoop` disassembly corrected the caller chain. `+0x110` is a Particle no-op; `+0x104` reaches `ObjectClass::DrawIt`, which dispatches `+0x114` to `0x0062CEC0`.

After those corrections, both roots, their load-bearing callees, the caller/vtable chain, all field offsets, and every boundary table were reread once more. That repeated pass added zero new open questions. It did catch and close the vtable-slot ambiguity before this report was written.

## Coverage ledger

Status describes the planned scope, not total semantic documentation of every helper's unrelated callers.

| # | Address | Planned identity | Status | Active in YR | Evidence/result |
|---:|---:|---|---|---|---|
| 1 | `0x0062C6E0` | Spark particle AI | verified | Yes | full root decompile/disassembly, field dataflow, branch matrix, three traces |
| 2 | `0x0062CE40` | Particle AI dispatch | verified | Yes | behavior-3 dispatch, signed lifetime decrement, zero equality |
| 3 | `0x0062E840` | PSC Spark AI | verified | Yes | forward AI and reverse dead cleanup relevant order |
| 4 | `0x0062B5E0` | Particle constructor | verified | Yes | relevant coordinate, velocity, color, lifetime, and vtable bindings |
| 5 | `0x00437090` | unknown helper | verified | Yes | three-dword copy/coordinate construction |
| 6 | `0x0043A100` | unknown helper | verified | Yes | `f32` Vec3 in-place addition, not matrix initialization |
| 7 | `0x006D6AD0` | unknown helper | verified | Yes | candidate-cell terrain slope-byte query |
| 8 | `0x007559B0` | `VXL_GetFacingMatrix` | verified | Yes | copies slope-indexed 3x4 matrix; local facing label is misleading here |
| 9 | `0x005AFC20` | unknown helper | verified | Yes | inverse orthonormal 3x4 matrix |
| 10 | `0x0043A0B0` | unknown helper | verified | Yes | three-dword vector set, not rotation |
| 11 | `0x0043A0D0` | unknown helper | verified | Yes | `f32` Vec3 scalar multiply; literal 1.0 |
| 12 | `0x005AF4D0` | unknown helper | verified | Yes | 3x3 matrix-vector multiply; translation ignored |
| 13 | `0x00578080` | `CellClass::GetGroundHeight` | verified | Yes | signed cell conversion and terrain-height source |
| 14 | `0x00565730` | map cell lookup | verified | Yes | truncation-toward-zero `/256`, dummy invalid cell |
| 15 | `0x0047C520` | building lookup | verified | Yes | first linked `WhatAmI==6` object while game active |
| 16 | `0x00480510` | wall-connectable query | verified | Yes | sentinel `(-1,-1)` recognizes overlay indices `2/0x1A/0xF3` |
| 17 | `0x0041C230` | `CoordStruct::Set` | verified-correction | Yes generally; not called here | three dword stores; no Spark-root xref, so not the final commit helper |
| 18 | `0x0062CEC0` | Particle draw root | verified | Yes | full point path, gates, A/Z/color/packing/surface write |
| 19 | `0x006D2140` | `CoordsToClient2` | verified | Yes | signed isometric projection, offsets, Z adjustment |
| 20 | `0x006D20E0` | `AdjustForZ` | verified | Yes | exact `z*multiplier`, threshold, bias, `Math__ftol` mechanism |
| 21 | `0x004114B0` | CircBuf scanline pointer | verified | Yes | root argument order, origin subtraction, wrap/lock contract |
| 22 | `0x007BD130` | Z scanline pointer | verified | Yes | `ushort` sample pointer and circular row behavior |
| 23 | `0x007BAEB0` | DSurface point writer | verified | Yes | RTTI/vtable binding, lock/pointer, 16/8-bit write, unlock |
| 24 | `0x0055AF60` | performance helper | verified | Yes | hysteresis and unsigned root comparison domain |
| 25 | `0x005865E0` | optional fog predicate | verified | Conditional/default-off | returns zero in active binary; only conditional caller gate |
| 26 | `0x005F4CF0` | `ObjectClass::DrawIt` | verified-correction | Yes | function starts `0x005F4B10`; planned address is interior `+0x114` dispatch |
| 27 | `0x006D8DB0` | tactical object loop | verified | Yes | relevant `+0x104 → DrawIt → +0x114` chain and layer context |
| 28 | `0x006D3D10` | tactical draw | verified | Yes | Z clear, terrain/layers, object loop, later persistent-light ordering |
| 29 | `0x006D2B60` | tactical Z dirty clear | verified | Yes | caller to actual dirty row-fill |
| 30 | `0x007BCF50` | planned Z rect clear | verified-correction | Yes through corrected callee | not the dirty fill; actual row-fill is `0x007BCFB0` |

## Open-question final log

There are no `OPEN` items. Deferred items require runtime evidence unavailable in this session.

| ID | Question | Status | Resolution/evidence or required next probe |
|---|---|---|---|
| F01 | Which fields are old position? | RESOLVED | signed `i32` world leptons at `+0x9C/+0xA0/+0xA4` |
| F02 | Which fields are velocities? | RESOLVED | `f32` at `+0x10C/+0x110/+0x114` |
| F03 | Is gravity applied once or twice? | RESOLVED | once to persistent Z velocity, twice in candidate probe |
| F04 | Which coordinate selects cell/ground? | RESOLVED | candidate coordinate; old cell additionally participates in bridge-flag test |
| F05 | How do signed world coordinates become cells? | RESOLVED | truncation toward zero by 256 |
| F06 | What does cell bit `0x100` mean here? | RESOLVED | active structural/high-bridge body/deck flag |
| F07 | What is bridge plane? | RESOLVED | candidate ground plus 416 leptons |
| F08 | What are crossing inequalities? | RESOLVED | descending `< / >=`; ascending `>= / <` as tabulated |
| F09 | What is ground clamp boundary? | RESOLVED | clamp only when `G-100 < newZ` |
| F10 | What is building/wall band? | RESOLVED | `[G,G+150)` |
| F11 | Which building is selected? | RESOLVED | first linked object with `WhatAmI==6` |
| F12 | What is `BuildingType+0x16BF`? | RESOLVED | parsed `LaserFence=` byte |
| F13 | What is the frame-state exception? | RESOLVED | laser-fence frame/connectivity `+0x618 >= 8` |
| F14 | What is building virtual `+0x80`? | RESOLVED | 1x1 `UndeploysInto` predicate |
| F15 | What does wall query `(-1,-1)` mean? | RESOLVED | direction-independent overlay index membership |
| F16 | Is slope matrix selected by facing? | RESOLVED | no; selected by candidate-cell terrain slope byte |
| F17 | What are the matrix helper roles? | RESOLVED | copy/add/invert/set/scale/3x3-multiply as ledgered |
| F18 | Is reflected impact velocity persistent? | RESOLVED | no; stack-local only |
| F19 | What collision state persists? | RESOLVED | coordinate, already-updated Z velocity, delete byte |
| F20 | Does collision consume color RNG? | RESOLVED | yes, after coordinate/delete work |
| F21 | When is deletion observed? | RESOLVED | reverse cleanup in same owning Spark-system tick |
| F22 | What if lifetime starts at zero? | RESOLVED | decrements to `-1`; no lifetime equality deletion |
| M01 | What is terrain level height? | RESOLVED | 104 leptons from active runtime and initializer |
| M02 | What is structural bridge offset? | RESOLVED | 416 leptons from the Particle-owned initializer |
| M03 | Is this subterranean/Tube logic? | RESOLVED | no |
| P01 | What is exact point caller chain? | RESOLVED | tactical `+0x104` → `ObjectClass::DrawIt` → Particle `+0x114` |
| P02 | Is Particle `+0x110` the draw call? | RESOLVED | no; it targets a `RET 8` stub |
| P03 | What are projection formulas? | RESOLVED | signed integer isometric terms plus data-driven Z adjustment |
| P04 | What is active numeric Z projection multiplier? | DEFERRED | attach a running retail session and read `g_AdjustForZ_Multiplier`; mechanism is already resolved |
| P05 | What are clip inclusivities? | RESOLVED | left/top inclusive, right/bottom exclusive |
| P06 | What is A sample width/signedness? | RESOLVED | full zero-extended `ushort` |
| P07 | What are A thresholds? | RESOLVED | zero rejects; `1..126` scale with `>>7`; `>=127` unchanged |
| P08 | What is Z candidate width/signedness? | RESOLVED | wrapped `u16` base, then signed `i32` subtractions |
| P09 | What is stored Z width/signedness? | RESOLVED | zero-extended `ushort` |
| P10 | What is the Z inequality? | RESOLVED | strict `candidate < stored`; equality rejects |
| P11 | Does point path write Z? | RESOLVED | no |
| P12 | What is the dirty clear value/helper? | RESOLVED | `0xFFFF` at `0x007BCFB0`; plan address corrected |
| P13 | How is index-zero color selected? | RESOLVED | per-particle RGB, next is `ColorList[1]` |
| P14 | How are nonzero colors selected? | RESOLVED | current list index, next list index plus one, three-byte stride |
| P15 | What is malformed-index behavior? | RESOLVED | unchecked native memory access |
| P16 | How are channels rounded/clamped? | RESOLVED | `f64` interpolation through `Math__ftol`; no clamp |
| P17 | How are channels packed? | RESOLVED | runtime Loss/Shift arithmetic and OR formula |
| P18 | What are active retail Loss/Shift numeric values? | DEFERRED | capture globals in a running retail display mode; do not assume RGB565 |
| P19 | What method writes the pixel? | RESOLVED | DSurface vtable `+0x24 -> 0x007BAEB0` |
| P20 | Can the surface still suppress? | RESOLVED | null pointer/lock/bounds failure yields no write |
| P21 | Is there a later native fullscreen A multiply? | RESOLVED for scoped chain | no second A multiply of the already-written Spark point was found |
| P22 | What certifies final packed pixel? | DEFERRED | runtime DD-mask capture plus breakpoint/oracle around `0x0062CEC0`/`0x007BAEB0` and primary-surface pixel readback |
| R01 | Is current Rust R8 A path exact? | RESOLVED verdict | DRIFT/UNCHECKED; wrong width and composition timing, no full-space proof |
| R02 | Is current `Depth32Float` path exact? | RESOLVED verdict | DRIFT/UNCHECKED; native uses wrapped integer candidate and strict `ushort` comparison |
| R03 | Can current SHP particle path be reused? | RESOLVED verdict | DRIFT; behavior-3 native mechanism is a direct point write |

Deferred ratio is 3 of 47 items (6.4%), below the investigation threshold. None blocks the static implementation mechanism; all three block only runtime numeric/final-pixel certification.

## Rust-facing implementation handoff

This is not an implementation contract and authorizes no code change. It states the verified deltas a later contract must turn into exact requirements.

| Area | Native requirement | Current Rust evidence | Verdict / required handoff |
|---|---|---|---|
| Spawn/state | Spark particles must carry signed world-lepton coordinates, three `f32` velocity components, per-particle RGB, signed index, `f64` accumulator, signed `i16` lifetime, and delete byte semantics | `spawn.rs` rejects Spark/Railgun; Rust uses `IVec3`, fixed-point direction/scalar velocity, drift fields, RGB/index/accumulator abstractions | MISSING/DRIFT; prove a Rust-native state mapping that preserves every native conversion and byte-visible result |
| AI dispatch/order | forward particle AI, collision commit, color RNG, signed lifetime decrement, reverse cleanup | system AI currently no-ops Spark/Railgun | MISSING; preserve exact owner iteration and RNG consumption |
| Gravity/motion | persistent `old_vz-g`, candidate probe `old_vz-2g`, `f32` addition, `Math__ftol` component conversion | simulation math uses fixed point by architecture rule | DRIFT until exact equivalence is positively proved; a contract must reconcile the project fixed-point rule with native `f32`/x87-visible outputs |
| Terrain/cell | truncation-toward-zero `/256`, candidate-ground query, 104-lepton levels, slope byte | resolved terrain/map surfaces exist | UNCHECKED; add the smallest read-only sim query with exact frames and signed conversion |
| Bridges | old/new structural bit, plane `ground+416`, asymmetric crossing snaps | bridge state/topology surfaces exist | UNCHECKED; query active structural/deck state without render dependency or Tube/subterranean conflation |
| Occupancy | first linked building order, laser-fence and 1x1 exceptions, sentinel wall overlays | occupancy/entity store exist but ordering equivalence is unproved | DRIFT/UNCHECKED; do not replace first-match semantics with unordered presence |
| Collision result | coordinate commit plus delete byte; no bounce/impact velocity write | no Spark implementation | MISSING; avoid inventing persistent reflected velocity |
| Point projection | exact native integer isometric conversion, data-driven Z multiplier, offsets, clip | app particle builder uses sprite presentation | MISSING/DRIFT; a distinct point compositor is required |
| A buffer | sample native-equivalent `ushort` at point time; zero reject; `1..126` integer scale; `>=127` pass-through | `ShroudBuffer` is R8 and later fullscreen multiply | DRIFT; current width/timing cannot cover native input space and can double-darken |
| Z buffer | wrapped 16-bit base, signed candidate, strict compare to zero-extended 16-bit stored value; read-only | scene depth is `Depth32Float`; particles have no depth interaction | DRIFT/UNCHECKED; needs an integer-compatible tactical depth substrate or exhaustive equivalence proof |
| Color | index-zero per-particle RGB, list-index rules, `f64` interpolation, `Math__ftol`, no clamp | partial particle color state exists | UNCHECKED; preserve list stride/index boundary and randomized-start transition |
| Packing/write | runtime DD-derived Loss/Shift packing and one DSurface point write | GPU render target format/pipeline differs | DRIFT/UNCHECKED; select a Rust-native presentation that proves identical final channel/pixel output for the supported retail mode |
| Frame order | Z clear/population, object point draw, later persistent lights; no second A modulation | particles draw in step 7.6 and fullscreen shroud multiply occurs later | DRIFT; contract must eliminate double modulation and reproduce native point-time buffer state |

The main architecture constraint remains: `sim/` must not depend on `render/`, `ui/`, `sidebar/`, `audio/`, or `net/`. Collision queries should be Rust-native read-only simulation interfaces backed by map/terrain/bridge/occupancy state. The compositor belongs above `sim/` but must consume a deterministic render snapshot containing the exact coordinate, color state, and native-compatible A/Z inputs.

Do not use the existing SHP atlas, a white 1x1 sprite, map-cell lighting, or a generic GPU depth test as substitutes unless an exhaustive proof shows identical state and pixels for the full relevant input space.

## Required executable/native acceptance path

A separate implementation-contract should require at least these gamemd-derived checks:

1. Launch the retail YR binary in a controlled display mode and record `g_DD_*Shift/Loss` plus `g_AdjustForZ_Multiplier` after DSurface/tactical initialization.
2. Break at `0x0062CEC0` for a stock `WeldingSpark`, capture particle fields, projected point, A word, Z word, computed candidate, interpolated channels, and packed value.
3. Break or watch at `0x007BAEB0`, confirm the same point/color reaches the active primary DSurface, then read back the destination pixel.
4. Exercise A samples `0,1,126,127,128`, Z samples candidate-minus-one/equal/plus-one, and all four clip edges in a native predicate oracle.
5. Compare the Rust result to those native captures. Rust-vs-prior-Rust images or hand-computed values may be regression fixtures but cannot certify parity.

## Corrections and reconciliation notes

- Replace any wording that says Spark collision stores a reflected impact/bounce velocity. It does not; the transform is stack-local and deletion is marked.
- Replace the plan's `ParticleClass` draw callsite claim at virtual `+0x110`. Particle point draw is virtual `+0x114`, reached through Particle `+0x104 -> ObjectClass::DrawIt`.
- Keep ParticleSystemClass `+0x114 -> 0x0062E280` separate from ParticleClass `+0x114 -> 0x0062CEC0`.
- Replace `0x007BCF50` as the dirty Z clear body with `0x007BCFB0`; retain the former only as a neighboring wrapper/helper identity.
- Replace a generic building gate description for `BuildingType+0x16BF` with the verified `LaserFence=` meaning and conditional stock reachability.
- Use `ParticleTypeClass+0x2C8` for active ColorList count. `+0x2C4/+0x2C5` are vector flags, not count.
- Use `ParticleTypeClass+0x2E0` for `MaxEC`; `+0x2DC` is `MaxDC`.
- Do not treat `VXL_GetFacingMatrix`'s local label as evidence of particle facing. The Spark call indexes it with the terrain slope byte.

## Sources

- Live Ghidra MCP, program `gamemd.exe`: fresh decompile/disassembly/read-memory/xref evidence for all Coverage Ledger entries, especially roots `0x0062C6E0` and `0x0062CEC0`.
- Live Ghidra RTTI/vtable evidence: `ParticleClass` vtable `0x007EF954`; `ParticleSystemClass` vtable `0x007EFB9C`; `BuildingClass` vtable `0x007E3EBC`; DSurface vtable `0x007E85D4`.
- Live Ghidra caller evidence: `ObjectClass::DrawIt @ 0x005F4B10`, virtual call at `0x005F4CFD`, and `Tactical_ObjectRenderingLoop @ 0x006D8DB0`.
- Live Ghidra x87 startup/conversion evidence: entry `0x007CD80F`, initializer dispatch `0x007CBDAF -> 0x007C8F46 -> 0x007CEAAF`, control-word mapping `0x007CBF14/0x007CC01C`, `WinMain` calls at `0x006BBFC1/0x006BBFC9`, saved-word capture `0x007C5EE4`, and `Math__ftol @ 0x007C5F00`.
- `docs/research/PARTICLESYSTEMCLASS_GHIDRA_REPORT.md`.
- `docs/research/PARTICLE_TIMING_SPARK_RAILGUN_NORMALIZED_GHIDRA_REPORT.md`.
- `docs/research/SPARK_LIGHT_EFFECT_TICK_ROUNDING_AND_FIRST_VISIBLE_STAGE_RESWARM_20260528.md`.
- `docs/research/BSURFACE_CIRCBUF_ABUFFER_REPORT.md`.
- `docs/research/ZBUFFER_DEPTH_SYSTEM.md`.
- `docs/research/building-selection-brackets/PRIMARY_SURFACE_ZBUFFER_BRACKET_OWNERSHIP_GHIDRA_REPORT.md`.
- `docs/research/building-selection-brackets/SURFACE_DRAWLINE_ABUFFER_ZTEST_PIXEL_CONTRACT_GHIDRA_REPORT.md`.
- `docs/research/skirmish-ui/SKIRMISH_PREVIEW_SURFACE_VTABLE_AND_CLIPPING_GHIDRA_REPORT.md`.
- Stock merged INI authority: `ini/rules.ini`, patched by `ini/rulesmd.ini`.
- Current Rust read-only navigation: `src/sim/particles/`, `src/map/resolved_terrain.rs`, `src/sim/bridge_state/`, `src/sim/occupancy.rs`, `src/app_instances/particles.rs`, `src/render/shroud_buffer.rs`, `src/app_render/draw_passes.rs`, and `src/render/batch.rs`.
