# Phase 3 Spark shared-dummy routing contract

Date: 2026-08-27
Scope: GSI-04.03, behavior-3 `Spark` only
Binary: active retail Yuri's Revenge `gamemd.exe` in live Ghidra (`/gamemd.exe`)
Method: exhaustive-slice re-investigation; live decompile plus assembly, active retail INI, current Rust read
Status: **VERIFIED implementation handoff; Rust row remains OPEN**

## Question and stop condition

What exactly happens when `ParticleClass` construction or behavior-3 Spark AI
queries a world coordinate whose fixed-512 linear cell is invalid, whose slot is
null/unallocated, or whose noncanonical axes alias a materialized slot? Establish
the ordered lookup transcript, every `CellClass` field Spark consumes, shared
dummy mutation/restamping, delete and RNG consequences, reachability, lifecycle,
and the smallest architecture-correct Rust delta.

Stop after the complete Spark-facing contract and evidence-backed exclusions are
closed. Do not expand into unrelated CellClass consumers or implement Rust here.

## Verdict first

The active binary **never treats a Spark cell miss as absence or error**. Both
`CellClass::GetGroundHeight @ 0x00578080` and
`MapClass::Get_CellClass_At_Coord @ 0x00565730` select the same process-global
dummy `CellClass @ 0x00ABDC50`, stamp only its packed coordinate at `+0x24`
(`0x00ABDC74`), and continue. Spark then consumes the dummy's persistent level,
slope, raw structural-bridge flag, overlay identity, and empty object-list
behavior in a branch-dependent order. Later lookups can restamp the same object
before retained-pointer reads.

Current Rust's shared substrate is sufficient in shape but Spark bypasses it.
`SparkCollisionWorld` performs strict real-cell lookups, errors on invalid or
unallocated cells, queries old before candidate, evaluates both bridge cells
without native short-circuiting, gathers building/overlay/slope eagerly, and
turns a constructor miss into `None`. That changes dummy state, collisions,
coordinates, deletion, lifetime, and the synchronized particle RNG stream.

This is not safely excludable. A valid allocated Size-diamond edge cell has an
adjacent unallocated canonical slot; behavior-3 motion has no interior-margin or
playfield clipping gate and stock types author signed horizontal velocity. An
edge Spark can therefore cross into the null slot. Truly out-of-capacity linear
indices need a coordinate near a fixed-array boundary and are not established as
an ordinary shipped-map occurrence, but they use the identical native branch and
do not change the implementation requirement.

## 1. Evidence sources and supersession

Primary live evidence:

- `ParticleClass__Constructor @ 0x0062B5E0`
- behavior-3 particle AI `0x0062C6E0`
- `CellClass::GetGroundHeight @ 0x00578080`
- `MapClass::Get_CellClass_At_Coord @ 0x00565730`
- `CellClass::ComputeGroundHeightAtCoord @ 0x0047B3A0`
- slope projection helper `FUN_006D6AD0`
- `Look_up_building_in_cell @ 0x0047C520`
- `CellClass::IsWallConnectableInDirection @ 0x00480510`
- `CellClass__Constructor @ 0x0047BBF0`
- `CellClass::AddContent @ 0x0047E8A0`
- `TechnoClass__EnterCell_AddToMultiCells @ 0x005683C0`
- `OverlayClass__Mark @ 0x005FC570`
- `MapClass__Resize @ 0x00565C10`

Retail data:

- `ini/rulesmd.ini`: `[General] Gravity=6`.
- Active systems include `SparkSys`, `WeldingSys`, `FirestormSparkSys`, and
  `LGSparkSys`; `DamageParticleSystems` and `DefaultSparkSystem` actively route
  stock combat to Spark.
- Stock horizontal moduli are `10`, `16`, `16`, and `13`; `MinZVelocity=40`,
  `ZVelocityRange=15`; `MaxEC=500` for all four Spark particle types.

Earlier corrected Spark collision and height reports were used only as leads.
Any older claim that the ground scalar is 90, the structural offset is 360, or
Spark is inactive is superseded here: active YR uses 104 leptons per level,
416 leptons for the structural bridge plane, and production Rust now dispatches
Spark.

## 2. World-to-cell and dummy contract

### 2.1 Both helpers use signed truncation and a fixed 512 stride

For each world/lepton component, both helpers compute signed division by 256
with truncation toward zero:

```text
cell_x = (x + ((x >> 31) & 255)) >> 8
cell_y = (y + ((y >> 31) & 255)) >> 8
linear = cell_y * 512 + cell_x
```

Neither helper independently bounds `cell_x` or `cell_y`. A noncanonical pair
whose signed linear result is admitted aliases that canonical table slot.

At `0x005780B9..0x005780CE`, `GetGroundHeight` rejects a negative linear index,
an index at least global map capacity `DAT_0087F928`, or a null slot under
`g_CellArray_Base`. At `0x0056575C..0x00565771`, the world-coordinate Map helper
performs the same tests against `Map+0x140` and `Map+0x13C`.

On either miss:

1. the converted `cell_x/cell_y` are narrowed to signed 16-bit words;
2. their exact packed dword is written to dummy `+0x24` at `0x00ABDC74`;
3. the helper selects/returns `0x00ABDC50`;
4. no other dummy field is cleared.

A real hit leaves the dummy untouched. Invalid linear and null-slot misses are
observationally identical after selection.

### 2.2 Ground evaluation continues on the dummy

`GetGroundHeight` calls `ComputeGroundHeightAtCoord @ 0x0047B3A0` after either
selection. That evaluator reads:

- signed byte `Cell+0x11B` as base level and multiplies it by 104;
- byte `Cell+0x11C` as the slope-table selector;
- the original low lepton bytes of X/Y for slope interpolation.

Thus a miss is **not** zero/no-ground unless the dummy currently has constructor
defaults. Persistent dummy level or slope changes the returned height. Valid
retail slopes are 0..20; malformed larger dummy slope values inherit native's
unsafe table behavior and are outside the safe authored-state boundary.

## 3. Particle constructor ordering

For behavior 3, `ParticleClass__Constructor @ 0x0062B5E0` executes:

1. one lifetime RNG draw at `0x0062B870`, then the native 16-bit lifetime add;
2. copy the input coordinate to its local coordinate;
3. first `GetGroundHeight(input)` at `0x0062B8B1`;
4. if `input_z <= first_ground`, call the same helper again at `0x0062B8C2`
   and assign that second result to local Z;
5. `Set_Raw_Coords @ 0x0062B8D2`;
6. only later, take the conditional start-color draw at `0x0062BAC0` when the
   authored color endpoints require interpolation.

The behavior-1-only draw at `0x0062B7F2` is excluded because Spark behavior is
3. A constructor miss therefore stamps its cell once, or twice with the same
coordinate when the clamp branch is taken. There is no RNG or other lookup
between those two stamps. The miss itself does not delete or reject creation.

## 4. Exact behavior-3 AI lookup transcript

The live assembly at `0x0062C6E0` establishes this order after the persistent-Z
write and candidate-coordinate arithmetic:

1. **Candidate ground** — call `GetGroundHeight(candidate)` at `0x0062C7D4`.
2. **Candidate CellClass** — call `MapClass::Get_CellClass_At_Coord(candidate)`
   at `0x0062C7F4`; retain its pointer in `ESI`.
3. Read candidate raw `Cell+0x140 & 0x100` at `0x0062C80A`.
4. **Short-circuit old CellClass** — only when the candidate structural bit is
   clear, call the Map helper for the old coordinate at `0x0062C81C` and read
   old `Cell+0x140 & 0x100`. Candidate structural true skips the old lookup.
5. Resolve bridge-plane crossing from the short-circuit OR.
6. Only when there was no bridge collision and candidate Z is in the contact
   band `[ground, ground+150)`, call `Look_up_building_in_cell(ESI)` at
   `0x0062C883`; if it returns no accepted building, call
   `ESI->IsWallConnectableInDirection(-1,-1)` at `0x0062C894`.
7. Only when any collision kind is selected, call `FUN_006D6AD0(candidate)` at
   `0x0062C95A`. That helper performs another world-coordinate Map lookup and
   then reads `Cell+0x11C` for the reflection matrix.
8. Set the collision delete byte at `0x0062CA34`, commit coordinates, then take
   exactly one color RNG draw at `0x0062CA86`; lifetime processing follows.

There is no lookup failure branch anywhere in this transcript.

### 4.1 Observable shared-dummy restamping

If candidate ground misses, it first stamps candidate. Candidate Cell lookup
then stamps candidate again. After that:

- candidate structural true: old lookup is skipped; the dummy remains stamped
  candidate until some later collision-slope lookup (which stamps candidate
  again);
- candidate structural false: old lookup runs; if old misses, the one dummy is
  restamped old even though `ESI` still points to the candidate lookup's result;
- building and wall checks use retained `ESI` without another lookup; if both
  lookups missed, they inspect the same dummy object after the old-coordinate
  restamp;
- a selected collision performs the slope helper and restamps candidate;
- no collision performs no slope lookup, so the last stamp remains candidate or
  old according to the short-circuit branch above.

The coordinate stamp does not alter level, slope, overlay, flags, or list state.

## 5. Every dummy field Spark consumes

| Field / state | Spark use | Native miss behavior | Rust implication |
|---|---|---|---|
| `+0x24` packed coord | side effect; retained helpers can observe last writer | stamped on every miss, never on real hit | preserve exact call/short-circuit order through the shared handle |
| signed `+0x11B` level | candidate ground evaluator | persistent; constructor default 0 | use live dummy snapshot, not `None` |
| `+0x11C` slope | candidate ground and collision-only matrix lookup | persistent; constructor default 0; collision lookup restamps candidate first | compute ground from the selected live cell; delay matrix lookup until collision |
| raw `+0x140 & 0x100` | candidate then conditional old structural bridge | persistent; constructor clears low 23 bits | read from the same fallback result; do not require live bridge runtime state for a dummy |
| `+0xE4` ground object-list head | first-building scan | proven permanently null for supported active lifecycle | return no building for dummy; no dummy list model needed for Spark |
| `+0x44` overlay identity | `IsWallConnectable(-1,-1)` | constructor `-1`; persistent and writable in the global object | wall iff live value is exactly 2, 26, or 243; do not route miss to rectangular OverlayGrid |

No other CellClass field is read by this Spark path. In particular, passing
`targetOverlay=-1` and `direction=-1` makes `IsWallConnectableInDirection`
read only `+0x44`; it does not inspect overlay data, owner, or object lists.

### 5.1 Object-list exclusion is closed

`Look_up_building_in_cell @ 0x0047C520` scans the ground list at `Cell+0xE4`,
following `Object+0x30`, and returns the first object whose `WhatAmI()==6`.
`CellClass__Constructor` initializes dummy `+0xE4=0`.

The only direct call to `CellClass::AddContent @ 0x0047E8A0` is
`TechnoClass__EnterCell_AddToMultiCells @ 0x005683C0`. Before that call, the
caller checks `linear >= 0`, `linear < Map capacity`, and the slot pointer is
non-null. Failed admission skips `AddContent`; its later fallback-shaped block
does not bypass the identical admission. Therefore ordinary active object-list
maintenance cannot add an object to the dummy. Remove paths cannot populate an
empty list. Dummy building lookup is consequently false, not unknown.

### 5.2 Overlay and flags are persistent, not immutable

`CellClass__Constructor @ 0x0047BBF0` sets `+0x44=-1`, level/slope zero, and
clears the low 23 raw flag bits. Primary lookups never restore those defaults.

Active overlay/bridge code can write through never-null fallbacks. In
`OverlayClass__Mark @ 0x005FC570`, the bridge-family paths perform fixed
three-cell writes through `MapClass__Get_CellClass` and call
`CellClass__RecalcAttributes`; edge-adjacent misses therefore can persistently
change dummy overlay/derived flags. The live bridge table values read at
`0x008333D0`, `0x008333F0`, `0x00833418`, and `0x00833438` are bridge IDs
(`74,83,92,94,96,98,205,214,223,225,227,229` and nearby `+0..3` variants), not
Spark's wall IDs 2/26/243. Ordinary valid wall placement writes its primary
allocated cell; cardinal wall cleanup may read an edge dummy but does not copy a
wall ID into it. Thus constructor-fresh or valid edge-bridge contamination is
non-wall for Spark, while the architecture must still preserve shared persistent
overlay/flag state rather than synthesize an OverlayGrid cell.

Corrupt/off-map object injection could write other overlay IDs to the dummy, but
that is outside supported valid retail-format lifecycle; it is not needed to
justify the active null-slot route.

## 6. Miss collision, deletion, and RNG consequences

A miss selects facts; it is not itself collision, deletion, or RNG consumption.
The normal collision formula then runs against those facts:

- dummy ground may cause bridge, below-ground, building, wall, or no collision;
- dummy raw structural `0x100` participates in candidate-first short-circuit;
- dummy building result is false;
- dummy overlay supplies the exact three-ID wall predicate;
- when collision is selected, dummy slope selects the reflection matrix after
  the final candidate restamp.

Collision sets the deletion flag and commits the collision coordinate before
the one per-tick color draw. No collision commits the candidate. In both cases
the color draw and lifetime decrement still occur. Therefore converting a miss
to a Rust error is synchronization-visible: current Rust preserves only the
already-written persistent Z velocity, then returns before coordinate commit,
delete, color RNG, and lifetime.

## 7. Reachability and exclusions

### 7.1 Unallocated canonical slot: reachable

Map Resize materializes the Size diamond inside the fixed table, not every
canonical slot. Each valid Size-diamond rim cell has a neighboring canonical
slot whose table entry is null. Spark AI contains no allocation-mask check,
playfield clamp, or interior safety margin before applying horizontal velocity.

Stock behavior-3 types author signed X/Y remainder domains:

- Spark: `[-9,9]` on each horizontal axis;
- WeldingSpark and FirestormSpark: `[-15,15]`;
- LargeSpark: `[-12,12]`.

With stock `MinZVelocity=40`, range 15, and gravity 6, valid draws provide many
ticks of flight; an outward-moving Spark originating on or sufficiently near an
allocated rim crosses the 256-lepton cell boundary. Active stock damage and
default Spark producers have no rule requiring an interior map margin. This is
an ordinary supported-state mechanism, even though its frequency is edge-bound.

### 7.2 Fixed-array invalid linear index: conditional, same contract

Spark can move only a bounded number of cells during its flight. Reaching
`linear < 0` or `linear >= Map capacity` therefore additionally requires a
valid source near a fixed-array boundary (possible for a valid maximum-edge
retail-format map, not shown as an occurrence in the shipped-map set). Smaller
shipped maps are separated from the fixed-array boundary by far more than one
Spark flight. This incidence distinction is non-load-bearing: the native branch
and Rust implementation are exactly the same as a null-slot miss.

The attempted shipped-map size diagnostic was stopped before completion to
avoid turning a mechanism proof into a corpus audit. No claim is made that a
specific shipped scenario produces an out-of-capacity Spark. There is no such
uncertainty for the adjacent null-slot route.

### 7.3 Fixed-stride alias: must remain real

Because only signed `linear` is bounded, a noncanonical `(cell_x,cell_y)` can
alias an allocated canonical slot. That is a real hit: it neither stamps nor
uses the dummy. Spark must inherit `get_cellclass_fallback_leptons` semantics,
not add component bounds before selection.

## 8. Dummy lifecycle and persistence

The dummy is one fixed identity at `0x00ABDC50`. `MapClass__Resize @ 0x00565C10`
calls the constructor on it at `0x005670F2`. Scenario full initialization,
load-driven resize, and RMG initialization establish this reset boundary before
their gameplay state.

Between Resize calls, all modeled writes persist globally across callers and
ticks; a lookup writes only coordinate. Save iterates allocated slots through
the non-stamping allocation probe and the dummy is not an allocated table slot,
so its transient state is not serialized as a map cell. Load/resize reconstructs
it, after which misses during load or gameplay can establish new persistent
state. Rust's existing `SharedCellDummy` identity and
`reconstruct_for_map_resize` boundary match this lifecycle for its represented
fields.

## 9. Exact current Rust disparities

Current files were read in the active Phase 3 worktree.

1. `src/sim/particles/spark_spawn.rs:327-336` calls
   `SparkCollisionWorld::ground_height_at`; `src/sim/particles/spark_world.rs:117-119`
   converts any miss/error to `None`. Native returns dummy-derived ground and
   may make a second identical call/stamp on the clamp branch.
2. `spark_world.rs:73-81` looks up old before candidate and evaluates old before
   candidate. Native is candidate ground, candidate cell/flag, then conditional
   old cell/flag.
3. `spark_world.rs:122-146` rejects invalid or unallocated cells instead of
   using `cell_rect::get_cellclass_fallback_leptons` and the shared dummy.
4. `spark_world.rs:79-81` converts a structural real cell through live bridge
   runtime state; a native dummy raw `0x100` is consumed directly and cannot
   require a `BridgeRuntimeState` cell.
5. `spark_world.rs:82-107` eagerly gathers building, rectangular overlay, ground,
   and slope. Native gates building/wall by bridge result and contact band, and
   performs the slope lookup only after a collision.
6. `spark_world.rs:88-92` errors when candidate is outside OverlayGrid. Native
   reads retained `CellClass+0x44`, including the dummy.
7. `system_ai.rs:216-225` propagates the query error after
   `begin_particle_tick`; as documented in `spark.rs:484-488`, Rust then retains
   persistent Z but consumes no RNG and leaves coordinate/lifetime untouched.
   Native continues through normal commit/delete/color/lifetime.
8. `SharedCellDummy` currently represents coordinate, level, slope, and
   `+0x140 & 0x1180`. It does not represent overlay identity. That omission is
   harmless only if Spark deliberately uses the proven valid-lifecycle non-wall
   projection; it must not consult a rectangular OverlayGrid on fallback.

## 10. Smallest architecture-correct handoff

Keep arithmetic and world ownership where they are. Replace only Spark's eager
strict fact adapter with a native-ordered query transcript over the existing
shared substrate:

1. Add a Spark-local selected-cell view around
   `cell_rect::get_cellclass_fallback_leptons`, preserving `CellRef::Real` versus
   the one `SharedCellDummy` handle. It must expose signed level, slope, raw
   structural bit, and whether the result is dummy.
2. Make constructor ground lookup return a value for both real and dummy. Call
   it once for the comparison and again only on `z <= ground`; do not swallow a
   cell miss. Missing entire map fixtures may keep a clearly separate
   test-only/unsupported-world policy.
3. In per-tick Spark, execute candidate ground first; select candidate cell and
   read candidate raw structural; select old only when candidate structural is
   false. Do not prefetch both.
4. Gate building/wall exactly behind no bridge collision plus the 150-lepton
   contact band. Dummy returns no building. Real cells retain the verified
   object-list ordering/filter adapter.
5. For wall, real cells may map `CellClass+0x44` from OverlayGrid; dummy must use
   shared dummy overlay state or the proven valid-lifecycle non-wall projection.
   The more reusable parity direction is to extend `SharedCellDummy` with signed
   overlay identity initialized to `-1` and let active overlay writers update it;
   do not create per-query dummy overlay snapshots.
6. Only after collision selection, reselect candidate through
   `get_cellclass_fallback_leptons` and read its live slope for the matrix. This
   is the required final candidate restamp.
7. Return normal `SparkCollisionFacts` and let the existing kernel preserve its
   verified commit/delete/color/lifetime ordering. Misses must not return a
   `SparkWorldError`.

Do not globally replace strict terrain access, add playfield clipping, component
bounds, or reconstruct the dummy on a miss. Those would alter other contracts.

## 11. Acceptance tests

Required focused tests should prove the transcript, not only final collision:

1. Constructor null-slot miss above ground: one candidate stamp, no clamp;
   below/equal: two same-coordinate stamps and clamp from persistent dummy
   level/slope; lifetime draw remains before and color draw after.
2. Candidate dummy structural clear, old dummy: final no-collision stamp is old;
   candidate retained pointer still observes shared non-coordinate state.
3. Candidate dummy structural set: old lookup is skipped and final stamp remains
   candidate when no later slope query occurs.
4. Collision from dummy facts: final slope lookup restamps candidate, collision
   deletes/commits, exactly one color draw occurs, and lifetime decrements.
5. No collision on a dummy: candidate commits, exactly one color draw occurs,
   lifetime decrements, and no slope restamp occurs.
6. Dummy object list behaves empty; dummy wall checks `-1`, 2, 26, and 243 if
   overlay state is modeled, with only the three IDs true.
7. Noncanonical fixed-stride alias to an allocated real cell is a real hit and
   leaves the prior dummy coordinate unchanged.
8. Null allocated-mask slot and negative/high linear misses follow the same
   normal Spark path rather than returning `SparkWorldError`.
9. Existing real-cell bridge/building/wall/collision tests continue to pass and
   additionally assert native query short-circuit order.

Suggested row gate:

```text
gsi_04_03_spark_shared_dummy_query_order_and_miss_continuation
```

## 12. Opening-question ledger

| Question | Result |
|---|---|
| Constructor ground calls and RNG order | Closed: one call, conditional identical second; lifetime before, optional color after |
| Fixed-512 miss selection/stamp | Closed for both helpers |
| Every Spark-consumed dummy field | Closed: coord, level, slope, structural bit, object-list empty, overlay identity |
| Subsequent restamps/order | Closed, including candidate structural short-circuit and collision-only slope restamp |
| Collision/delete/RNG on miss | Closed: no error branch; normal resolution and one color draw |
| Null-slot reachability | Closed: valid rim plus stock signed motion, no interior guard |
| True out-of-capacity shipped incidence | Non-load-bearing incidence not claimed; same verified implementation branch |
| Dummy object-list contamination | Excluded for supported active lifecycle |
| Overlay/flags persistence | Closed: constructor defaults, active fallback writers, no per-lookup reset |
| Save/load lifecycle | Closed: not an allocated serialized cell; reconstructed at Resize |
| Rust mismatch and handoff | Closed; row remains open pending implementation/tests/critic loop |

No load-bearing Spark/dummy mechanism remains approximate in this report.

## 13. Ghidra annotation candidates (not applied)

No Ghidra mutation was performed. Candidate comments for a later certainty-gated
metadata pass:

- behavior-3 `0x0062C6E0`: document candidate-ground -> candidate-cell/structural
  -> conditional old-cell/structural -> gated building/wall -> collision-only
  candidate slope lookup.
- `FUN_006D6AD0`: candidate name
  `MapClass__GetSlopeTypeAtWorldCoord`, noting that it reuses/stamps the shared
  dummy and reads `Cell+0x11C`.
- `Look_up_building_in_cell @ 0x0047C520`: note ground list `+0xE4`, first
  `WhatAmI()==6`, no fallback special case.

## 14. Validation performed

- Live Ghidra decompile and assembly re-read of every primary function named in
  sections 2-5.
- Program-wide instruction census of `MOV ... [reg+0x44]` writers, followed by
  focused live inspection of relevant Cell/Overlay/bridge paths.
- Raw retail bridge overlay table reads for the active IDs cited above.
- Current Rust direct read of `spark_world.rs`, `spark_spawn.rs`, `spark.rs`,
  `system_ai.rs`, `cell_rect.rs`, and `resolved_terrain.rs`.
- Retail `rulesmd.ini` read for active systems, producer routing, velocity,
  gravity, and lifetime domains.
- No Rust source edit, commit, push, or Ghidra mutation.
