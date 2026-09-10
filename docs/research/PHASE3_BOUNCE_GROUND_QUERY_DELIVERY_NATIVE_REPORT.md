# Bounce ground-query delivery — bounded Phase 3 increment

Status: bounded native/Rust increment validated. No Phase 3 row is closed.

The common Cell ground evaluator already uses the correct 104-lepton scalar.
This increment addresses its live VoxelAnim consumer's map queries and selected
416-lepton bridge surface. It does not complete GSI-05.14 debris physics.

## Established native behavior

Original executable SHA-256:
`1cdd1180e49024fbda8ad568caac2e86e856063ff67ab38f62b7d2c7bb84298c`.
Fresh read-only bodies and assembly were inspected in `gamemd.exe`.

- `439B00` obtains new-position ground through `578080` at `439BF3`, then
  retains the separate `565730` Cell pointer at `439C17`. It tests raw Flags100;
  only when absent does it query the old coordinate at `439C38`.
- Ground uses the full signed, truncating `/256` world lookup, fixed512 index,
  and retained shared dummy. It does not return zero merely because a cell is
  absent. The existing ground-height oracle already covers the inner evaluator.
- Building/wall lookup at `439CAB`/`439CC0` uses the retained new Cell identity.
  A later miss may have changed that same dummy's coordinates.
- `47C520` selects the first RTTI6 object on the normal Cell list while the
  game is active. `480510(-1,-1)` accepts raw overlays2,26,243. A cached
  navigation class is not equivalent: class5 also includes terrain objects.
- Building virtual80 resolves `457620 -> 465D40`: a non-null UndeploysInto
  type and a1x1 foundation suppress the contact. The LaserFence-dependent
  rejection (type+16BF and building+618>=8) is not covered; all compared
  building fixtures have that flag clear. No TS-only/unreachable claim is made.
- `6D6AD0` independently queries the candidate Cell for slope. Following
  contact, `439F3A` queries OLD first, then `439F50` queries recomputed post-snap
  NEW and sign-extends each level. `439A10` independently queries ground and
  then Cell structural flags for the stop decision.
- Original `439610` derives deck416 from initialized integer104 at89C778.
  The old Rust constant0 is incorrect. The separate
  [startup capture](../../tools/spatial_oracle/bounce_startup_capture.json)
  binds those values and FPCW E7F to the active original process; its full
  executable section matches the file bytes.

## Active-retail production path

Fresh extracted `rulesmd.ini` (winner `expandmd01.mix`, SHA-256
`3d341ef8a13a4b5ab24af2eef48ac94931ac2bb87d950fe3330a07e2d25672ef`)
contains `VoxelAnims` entry `2=TIRE`; CMIN and HARV both have TechLevel1,
MaxDebris6, DebrisTypesTIRE and DebrisMaximums4. TIRE authors Elasticity0.8,
MinAngularVelocity12, MaxAngularVelocity24, MinZVel28, MaxZVel32,
MaxXYVel10 and Duration150. The freshly extracted `artmd.ini` SHA-256 is
`e1f0378394313c04ebbd5073f47785ee3e46f1b3c62d65724e8f3c310ee7ba31`;
there is no TIRE override there. These are active YR miner types, not TS-only
terrain-meteor assumptions.

Fresh constructor xrefs confirm `TechnoClass::ReceiveDamage 701900` calls
`VoxelAnimClass::Constructor 7493B0` at `702397`. The death-block guards and
random budget are documented next to production `sim/voxel_anim.rs`; positive
MaxDebris, a nonempty DebrisTypes vector and a positive random budget admit the
voxel loop. Fresh `VoxelAnimClass::AI 749F30` reaches `439B00` for positive
remaining Duration. No claim that every death necessarily spawns a piece.

## Delivery boundary

The production adapter moves to `world/bounce_terrain.rs`, using existing CellRef,
live terrain, normal occupancy, rules and overlay authorities. Bounce retains a
selected Cell handle across the conditional old lookup. The VoxelAnim LogicVector
visit supplies live rules to the adapter. No new serialized map authority is added.

The native fixture runs original439B00 through its return, including original
ground, lookup, surface, reflection and stop helpers. Fixture map/objects and
initialized globals are explicit; no native instructions or function results are
replaced. Native output is bounded evidence, not a retail game-loop capture.

## Explicit exclusions and residuals

- Slope reflection, cliff rollback and quaternion integration are pre-existing
  GSI-05.14 physics/presentation work. Ordinary slope cells were already passed
  with their correct slope index before this adapter repair. They are not
  automatically Phase 3 prerequisites. Flat contact outcomes and airborne slope
  ground sampling can establish this increment without claiming those physics.
- Cliff rollback requires a signed level increase of at least2 plus velocity/Z
  conditions. Its missing response changes position, velocity and stop timing;
  those inputs are excluded from full-update parity claims.
- Raw slope bytes above20 address native process memory. Existing Rust fallback
  is not parity for that domain; it remains open.
- Adjacent VoxelAnim749F30 host queries remain unclosed: its bounced water test
  reads the previous Object coordinate, whereas the existing Rust host uses the
  advanced Bounce coordinate; native later damage-list and expiry queries are
  not represented. The comparison endpoint is439B00 return, not the host loop.
- This report does not certify all ground-height callers, the entire Bounce
  contact loop, or whole Phase 3. The earlier scalar90 claim is disproved.

## Validation

The saved [48-case native comparison](../../tools/spatial_oracle/bounce_height.json)
and [reproducible harness](../../tools/spatial_oracle/bounce_height.py) compare flat
contact outcomes, airborne slope0..20 ground samples, signed levels, bridge
composition, shared dummy state, normal object-list order, 1x1 undeploy exclusion
and the three wall IDs. Rust compares explicit native ground results, every
lookup/surface transcript, position bits, exact finite numerical velocity, return outcome and terminal
dummy coordinates. Native lookup traces are observed, not substituted.

`python -B -m tools.spatial_oracle.bounce_height --check`: exit0,47cases,
`.local/bounce-height-native-v2.log`. Fresh independent read-only critic reproduced
that result and passed the bounded source/fixture review.

The initial focused Rust run failed because its sparse fixture supplied cell(1,0)
as backing vector index0. The dense backing plus separate native-allocation mask
was corrected. That failure was a fixture error, not native divergence evidence.

The second focused run passed the production LogicVector visit and exposed the
pre-existing flat-reflection signed-zero difference: native deck-fall Vy is-0,
Rust is+0. The restored physics approximation is outside this spatial increment.
All18 contact fixtures have zero input Y velocity; three also have X velocity+/-8. The comparison retains native
velocity bits but excludes only the sign of zero from velocity equality, with no
numerical tolerance. Position bits, ground, query transcript and outcomes remain
exact. The test collects all case differences before failing to avoid hiding later
mismatches. Full flat-state bit parity is not claimed.

Focused v5 (`cargo test -p vera20k --lib sim::world::techno_ai::bounce_terrain::tests::`):
exit0,2passed,0failed,8719filtered; `.local/bounce-height-focused-v5.log`.
This includes all47 then-frozen native fixtures and the live VoxelAnim visit.
The final corpus adds one native-derived resting structural-deck case (Stopped)
and checks that the production host sets Duration0. Final required validation:

- `python -B -m tools.spatial_oracle.bounce_height --check`: exit0,48cases;
  `.local/bounce-height-native-final.log`. Set `VERA20K_GAMEMD_EXE` to the
  hash-bound original executable before running the harness.
- `cargo test -p vera20k --lib`: exit0,8602passed,0failed,119ignored;
 22.71s test runtime; `.local/bounce-height-full.log`. This includes the final
  48-case comparison and all five production VoxelAnim visit scenarios.
- `cargo clippy -p vera20k --lib`: exit0,1144warnings,1m05;
  `.local/bounce-height-clippy.log`. Existing warning backlog was not rebaselined.
- Fresh read-only critic independently reproduced the native comparison and
  reviewed the final resting-deck delta with no actionable finding.

No implementation changes followed these final Rust checks. The production
VoxelAnim test visits the actual live store slot for high/low bridges and the
building/undeploy distinction; it does not certify the entire host AI.

