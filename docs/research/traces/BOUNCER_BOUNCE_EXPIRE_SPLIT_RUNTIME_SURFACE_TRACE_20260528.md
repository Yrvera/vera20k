# Bouncer BounceAnim vs ExpireAnim Runtime Surface Trace - 2026-05-28

**Scenario:** A bouncer animation impacts terrain. Verify gamemd separates the `BounceAnim=` constructor row from the later `ExpireAnim=` constructor row with distinct args, then trace whether current Rust can represent and/or emit those rows after the metadata and generic descriptor changes.

**Scope:** One mechanic only: bouncer terrain impact `BounceAnim=` versus `ExpireAnim=` runtime surface.

**Hard constraints honored:** Ghidra MCP was read-only; no mutating Ghidra tools were used. This report is the only file written for this trace.

## Summary Verdict

Current Rust can parse `BounceAnim=` and `ExpireAnim=` into separate metadata fields and has a generic `AnimClassSpawnDescriptor` capable of carrying constructor args. Current Rust does **not** have the bouncer/meteor `AnimClass` runtime surface that would consume that metadata, run BounceClass-like impact logic, or emit either the `BounceAnim` row or the later `ExpireAnim` row.

The player-visible result is that terrain-impact debris/metors that gamemd resolves through bouncer impact animation rows cannot currently produce the native bounce/impact visual sequence from this mechanism. If a bouncer type has both keys, gamemd can create `BounceAnim` first with `drawFlags=0x600,zAdjust=0`, then `ExpireAnim` later in `AnimClass::AI` with `drawFlags=0x2600,zAdjust=-30`; Rust emits neither from a bouncer impact path.

Verdict tally: **PASS: 0 | FAIL: 0 | UNCHECKED: 2 | NOT-IMPLEMENTED: 5**

## Evidence Inputs

- `docs/research/ANIMCLASS_BOUNCER_METEOR_EXPIREANIM_IMPACT_SPAWNS_GHIDRA_REPORT.md`
- `docs/research/ANIMCLASS_CHAINING_DAMAGE_OWNERSHIP.md`
- `src/rules/art_data.rs`
- `src/sim/components.rs`
- `src/app_building_anim.rs`
- Focused `rg` scans over current Rust bouncer/AnimClass runtime surfaces.
- Read-only Ghidra spot-checks:
  - `AnimClass::ProcessBounceResult @ 0x00423930`
  - `AnimClass::AI @ 0x00423AC0`
  - `AnimClass::Destroy @ 0x004255B0`

## Standard YR Activation Check

The native path is active in standard YR, conditionally on constructed anim types with `Bouncer=yes` or `IsMeteor=true`. Retail `ini/artmd.ini` contains active examples:

- `[DBRIS1LG]`: `ExpireAnim=TWLT036`, `Damage=20`, `DamageRadius=80`, `Warhead=HE`, `Bouncer=yes` (`ini/artmd.ini:14986-14999`).
- `[METLARGE]`: `ExpireAnim=TWLT070`, `Damage=5000000`, `DamageRadius=300`, `Warhead=Meteorite`, `IsMeteor=true` (`ini/artmd.ini:19061-19069`).
- `[METSMALL]`: `ExpireAnim=TWLT100`, `IsMeteor=true` (`ini/artmd.ini:19082-19090`).
- `[METDEBRI]`: `ExpireAnim=TWLT070`, `Damage=40`, `DamageRadius=100`, `Warhead=TankOGas`, `Bouncer=yes` (`ini/artmd.ini:19104-19119`).

This confirms the `AnimClass::AI` bouncer/meteor branch and `ExpireAnim=` impact surface are live in standard YR. A stock `BounceAnim=` key was not found in `ini/artmd.ini` or `ini/art.ini`; the binary branch is active code but same-type stock emission of both rows was not confirmed from retail INI content in this trace.

## Pipeline

1. Data: art type carries bouncer/meteor flags and optional `BounceAnim=` / `ExpireAnim=` references.
2. Trigger: a constructed bouncer/meteor `AnimClass` hits terrain during its AI tick.
3. Native bounce update: `AnimClass::ProcessBounceResult` runs embedded `BounceClass::Update`.
4. Native `BounceAnim=` row: if update returns `1` and `type->BounceAnim` is non-null, gamemd constructs `BounceAnim`.
5. Native `ExpireAnim=` row: `AnimClass::AI` receives return `1` or `2`, accepts the terrain/water gate, and if `type->ExpireAnim` is non-null constructs `ExpireAnim`.
6. Native parent cleanup: `AnimClass::AI` calls `Destroy` after accepted impact processing. Normal `Destroy` itself does not construct `ExpireAnim`.
7. Rust metadata: `ArtRegistry` parses `BounceAnim=` and `ExpireAnim=`.
8. Rust runtime: no generic bouncer `AnimClass` / BounceClass driver emits either row.
9. Rust screen result: no bouncer-impact row reaches `WorldEffect` or another visible effect path.

## Stage Results

### Stage 1 - INI Metadata Surface

**gamemd:** `AnimTypeClass::ReadINI` stores `BounceAnim` at `+0x300`, `ExpireAnim` at `+0x304`, `Bouncer` at `+0x35A`, `IsMeteor` at `+0x356`, `Warhead` at `+0x330`, and `DamageRadius` at `+0x334` per the verified bouncer report.

**Rust:** `AnimTypeRuntimeConfig` has separate `bounce_anim` and `expire_anim` fields (`src/rules/art_data.rs:180-190`) and parses them separately (`src/rules/art_data.rs:274-288`). It does not carry `Bouncer`, `IsMeteor`, `Damage`, `DamageRadius`, or bouncer-impact `Warhead` in this runtime config.

**Verdict:** **NOT-IMPLEMENTED** for the full bouncer-impact data surface. The two animation-name fields exist, but the flags and damage/warhead fields required to activate and complete this native path are missing from the current generic runtime metadata.

### Stage 2 - Native BounceAnim Constructor Row

**gamemd:** Read-only Ghidra for `AnimClass::ProcessBounceResult @ 0x00423930` confirms that when bounce update returns `1`, gamemd reads `type+0x300` and constructs `BounceAnim` with:

`delay=0`, `loopCount=1`, `drawFlags=0x600`, `zAdjust=0`, `reverse=0`.

This is the same fact reported in `ANIMCLASS_BOUNCER_METEOR_EXPIREANIM_IMPACT_SPAWNS_GHIDRA_REPORT.md`.

**Rust:** No current Rust runtime consumer of `AnimTypeRuntimeConfig::bounce_anim` was found. Focused `rg` found only parser/test references in `src/rules/art_data.rs`.

**Verdict:** **NOT-IMPLEMENTED**. Rust emits zero `BounceAnim` constructor rows from bouncer impact; gamemd emits one when the native conditions are met and `BounceAnim` is non-null.

### Stage 3 - Native ExpireAnim Constructor Row

**gamemd:** Read-only Ghidra for `AnimClass::AI @ 0x00423AC0` confirms the `ExpireAnim` constructor is separate from `ProcessBounceResult`. After return `1` or `2`, accepted terrain/water gate, and non-null `type+0x304`, gamemd constructs:

`AnimClass(type->ExpireAnim, impact_coords, delay=0, loopCount=1, drawFlags=0x2600, zAdjust=-30, reverse=0)`.

`impact_coords` are converted from embedded BounceClass float position through `Math::ftol`.

**Rust:** No current Rust runtime consumer of `AnimTypeRuntimeConfig::expire_anim` was found. Focused `rg` found only parser/test references in `src/rules/art_data.rs`.

**Verdict:** **NOT-IMPLEMENTED**. Rust emits zero `ExpireAnim` constructor rows from accepted bouncer impact; gamemd emits one when the native conditions are met and `ExpireAnim` is non-null.

### Stage 4 - Row Separation and Ordering

**gamemd:** The split is verified:

- `BounceAnim=` is read from `type+0x300` inside `ProcessBounceResult @ 0x00423930`.
- `ExpireAnim=` is read from `type+0x304` later in `AnimClass::AI @ 0x00423AC0`.
- For return `1` on accepted terrain, the call order is `BounceAnim` first, then `ExpireAnim`, then parent destroy.
- Constructor args are distinct: `0x600,0` versus `0x2600,-30`.

**Rust:** Current Rust has no bouncer impact pipeline, so no runtime ordering exists for these rows. `AnimClassSpawnDescriptor` can hold a row's constructor fields (`src/sim/components.rs:769-789`), and `WorldEffect::from_anim_spawn` preserves a descriptor (`src/sim/components.rs:864-887`), but no bouncer code creates either descriptor.

**Verdict:** **NOT-IMPLEMENTED** for runtime ordering. Structural representation exists, runtime emission/order does not.

### Stage 5 - Normal Destroy Negative Fact

**gamemd:** Read-only Ghidra for `AnimClass::Destroy @ 0x004255B0` confirms the body detaches owner state, releases sound, optionally plays `StopSound` from `type+0x2FC`, and calls `ObjectClass::UnInit`. It does not read `type+0x304` and does not construct `ExpireAnim`. The existing decompiler pre-comment claiming `ExpireAnim` is stale; the function body refutes it.

**Rust:** No generic `AnimClass::Destroy` equivalent exists for bouncer anims. Existing app-side `AnimRuntime` expiry in `src/app_building_anim.rs:892-940` only expires/morphs garrison occupant anims and is not a bouncer destroy path.

**Verdict:** **UNCHECKED** for Rust parity. There is no bouncer destroy surface to compare; the native negative fact is verified.

### Stage 6 - Generic Descriptor Representation

**gamemd expected rows:**

- Bounce row: `delay=0`, `loopCount=1`, `drawFlags=0x600`, `zAdjust=0`, `reverse=0`.
- Expire row: `delay=0`, `loopCount=1`, `drawFlags=0x2600`, `zAdjust=-30`, `reverse=0`.

**Rust:** `AnimClassSpawnDescriptor` has numeric fields for `delay`, `loop_count`, `draw_flags`, `z_adjust`, and `reverse` (`src/sim/components.rs:769-789`). `WorldEffect::from_anim_spawn` stores the descriptor verbatim and converts delay to milliseconds (`src/sim/components.rs:864-887`). Teleport uses this row bridge for warp visuals (`src/sim/movement/teleport_movement.rs:56-76`), but no bouncer code uses it.

**Verdict:** **UNCHECKED** as a parity stage. The struct can represent the two rows, but no bouncer runtime output was computed, so this is representation capacity only, not a passing emitted result.

### Stage 7 - Screen Result

**gamemd:** An accepted impact can produce native visible animation objects: optional `BounceAnim` at the bounce event, optional `ExpireAnim` at impact coords with `drawFlags=0x2600` and `zAdjust=-30`.

**Rust:** Current visible `WorldEffect` producers for wake, combat death explosions, bridge effects, and superweapons either set `anim_spawn: None` or use a different mechanism; only teleport currently uses `WorldEffect::from_anim_spawn`. No bouncer-impact producer reaches screen.

**Verdict:** **NOT-IMPLEMENTED**. The player will not see the native bouncer impact animation sequence from this mechanism.

## Failures And Missing Pieces

1. **Stage 1 - Full bouncer metadata is missing.** Rust parses `BounceAnim` and `ExpireAnim`, but not the bouncer activation and impact-damage fields needed for the path. Evidence: `src/rules/art_data.rs:180-190`, `src/rules/art_data.rs:274-288`; gamemd fields in the verified bouncer report.
2. **Stage 2 - `BounceAnim` emit path is missing.** Rust has no consumer of `bounce_anim`; gamemd constructs `type+0x300` on bounce return `1` with `0,1,0x600,0,0`.
3. **Stage 3 - `ExpireAnim` emit path is missing.** Rust has no consumer of `expire_anim`; gamemd constructs `type+0x304` after accepted impact with `0,1,0x2600,-30,0`.
4. **Stage 4 - Same-tick row ordering is missing.** Rust cannot currently preserve `BounceAnim` first, `ExpireAnim` later, parent destroy last because there is no bouncer AI/impact pipeline.
5. **Stage 7 - Visible result is missing.** No Rust bouncer-impact row reaches `WorldEffect` or an equivalent renderer-visible anim list.

## Timing And Sequencing

Native timing for the traced path:

1. `AnimClass::AI` runs for the parent bouncer on tick `T`.
2. It calls vtable `+0x1E8`, reaching `ProcessBounceResult`.
3. On return `1`, `ProcessBounceResult` may construct `BounceAnim` during that same AI visit.
4. Control returns to `AnimClass::AI`.
5. If return `1` or `2` and the terrain/water gate accepts the landing, `AnimClass::AI` may construct `ExpireAnim` in the same AI visit.
6. `AnimClass::AI` calls parent `Destroy` after accepted impact handling.

Rust timing is **not implemented** for this mechanic. No current Rust stage computes the bouncer impact tick, the return code, the terrain gate, or the two-row same-visit ordering.

## Adjacent Findings

- Stock retail `ini/artmd.ini` and `ini/art.ini` scans did not find a `BounceAnim=` key. This does not make the binary branch dormant; it means the same-type `BounceAnim` row is a conditional runtime surface not confirmed from stock INI content in this trace.
- `AnimClass::AI` also contains water/splash and debris/tiberium-spawn branches near the bouncer impact logic. Those are adjacent and were not traced here.
- Existing `WorldEffect` producers frequently set `anim_spawn: None`, so the generic constructor-row migration is partial beyond this bouncer mechanic.

## Return Contract Summary

Report file: `C:/Users/enok/Documents/ra2-rust-game/docs/research/traces/BOUNCER_BOUNCE_EXPIRE_SPLIT_RUNTIME_SURFACE_TRACE_20260528.md`

Verdict tally: **PASS: 0 | FAIL: 0 | UNCHECKED: 2 | NOT-IMPLEMENTED: 5**

Top player-visible missing pieces:

1. Stage 7 - no bouncer-impact visual sequence reaches screen; Rust surface `src/sim/components.rs:823`; gamemd evidence `AnimClass::AI @ 0x00423AC0`.
2. Stage 3 - `ExpireAnim` impact row is never emitted; Rust parser-only field `src/rules/art_data.rs:287`; gamemd row `delay=0,loop=1,drawFlags=0x2600,zAdjust=-30,reverse=0`.
3. Stage 2 - `BounceAnim` bounce row is never emitted; Rust parser-only field `src/rules/art_data.rs:286`; gamemd row `delay=0,loop=1,drawFlags=0x600,zAdjust=0,reverse=0`.
4. Stage 4 - row order cannot be preserved; Rust descriptor exists at `src/sim/components.rs:769` but no bouncer emitter exists; gamemd order is `ProcessBounceResult` row before `AI` row.
5. Stage 1 - full bouncer activation metadata is absent; Rust config `src/rules/art_data.rs:180`; gamemd requires `Bouncer`/`IsMeteor` plus impact fields.

Status: **COMPLETE**
