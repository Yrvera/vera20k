# Facing Byte vs Direction Index - Ghidra Research Report

**Address(es):** `0x0049F2F0`, `0x006FDD50`, `0x00468000`, `0x005B20F0`, `Facing__GetTurnDelta`, `Facing__IsWithinROT`, `Facing__ClampToROT`, `0x007353C0`, `0x004C9680`, `0x00737430`, `0x004B0EF0`, `0x004B0F20`, `0x004B0C40`, `0x007559B0`
**Investigation Mode:** coverage-map
**Claimed Scope:** relation between 8-bit facing bytes, 16-bit FacingClass/DirStruct values, 0..7 cell direction indexes, 8-way/32-way quantization formulas, movement vector to facing, facing to movement vector, rotating SHP projectile frame mapping, current VXL-facing surface, and refinery miner dock-facing claims.
**Non-Scope:** full VXL draw-matrix caller chain, full infantry SHP sequence frame math, full DriveTrack table decode, runtime debugger traces for exact per-map miner approach-facing during dump, and Rust implementation changes.
**Confidence:** High for direction table, FacingClass helper arithmetic, `ROT` clamp/shift, Fire_At muzzle 8-way index, DRAGON 32-way projectile frame formula, and dock radio `0x16` not being a facing setter. Medium for current VXL-facing delta because this pass verified `VXL_GetFacingMatrix` indexing but did not exhaust all callers that choose the matrix index.
**Active in YR:** Yes for the verified gameplay/render paths; conditional where noted.

## 0. Investigation Frame

**Target question:** Do gamemd's 8-bit facing bytes and 0..7 direction indexes share the same compass origin/order, where is rounding performed, and which systems use raw facing, quantized direction, or a separate drive-track/rate-timer value?

**Non-goals:** Do not edit Rust or in-repo docs. Do not re-open every infantry/VXL renderer. Do not treat older report labels as ground truth when the decompile disagrees. Do not resolve exact dump-facing for every possible refinery approach path by static analysis alone.

**Evidence needed to mark COMPLETE:** direction table order; 8-bit to 16-bit conversion; 16-bit facing helpers; 8-way and 32-way quantization constants; at least one live movement-vector to facing path; at least one facing to movement-vector path; live render frame mapping for rotating SHP; dock/miner `0x4000` conflict resolved; current Rust deltas named.

**Stop conditions:** stop after the above contracts are proved or explicitly deferred; stop before VXL caller exhaustion if it becomes a separate renderer investigation; stop before any Rust patch.

## 1. Overview

The canonical adjacent-cell direction index is `0..7 = N, NE, E, SE, S, SW, W, NW`, populated at startup into `g_DirectionOffsets @ 0x0089F688`. The 8-bit facing byte uses the same compass origin and clockwise order for movement/render contracts: `0=N`, `64=E`, `128=S`, `192=W`; a 16-bit facing value is normally the high-precision form, with the 8-bit byte equal to the high byte for Rust-facing purposes.

The critical caveat is that not every `0x4000` in the binary is a facing. In the refinery dock radio `0x16` path, `0x4000` is passed to DriveLocomotion vtable `+0x4C`, whose concrete method is `DriveLocomotionClass__Do_Turn @ 0x004B0EF0` and only calls `RateTimer__Set`; it is not a body-facing target. Older dock-facing reports and current Rust comments that describe this as "pivot to East" are stale.

## 2. Class Layout / Key Offsets

| Owner | Offset / value | Meaning | Active in YR | Evidence |
|---|---:|---|---|---|
| global | `0x0089F688..0x0089F6A4` | eight adjacent cell offsets, index `0..7` | Yes | `0x0049F2F0`; prior `GDIRECTIONOFFSETS...` |
| `FacingClass` helper input | `short` | 16-bit signed wrapping facing math | Yes | `Facing__GetTurnDelta`, `Facing__IsWithinROT`, `Facing__ClampToROT` |
| `TechnoTypeClass` | `+0x71C` | `ROT=` value parsed from INI, used for unit facing-rate setup | Yes | `0x007353C0`, `0x004C9680`; `CMIN_RUNTIME_ROT...` |
| `FacingClass` rate field | `+0x14` in setter context | stores `(clamped ROT byte) << 8`; clamp is `>0x7E -> 0x7F` | Yes | `0x004C9680` |
| `BulletClass` | `+0xE8/+0xF0` | velocity X/Y consumed by rotating SHP frame helper | Yes | `0x00468000` |
| `BulletTypeClass` | `+0x2A1` | inverted `Rotates`; zero enables velocity frame mapping | Yes | `0x00468000`; DRAGON report |
| `DriveLocomotionClass` | `+0x4C` | residual/rate-timer field set by vtable `+0x4C` | Yes | `0x004B0EF0`, `0x00737430` |
| `DriveLocomotionClass` | `+0x54/+0x58` | drive track index and point index | Yes | `0x004B0C40`, `0x004B0F20` |

## 3. Core Logic

### 3.1 Direction index table

Verified direction index order:

| Index | Cell delta | 8-bit facing equivalent |
|---:|---:|---:|
| 0 | `(0,-1)` | `0` |
| 1 | `(1,-1)` | `32` |
| 2 | `(1,0)` | `64` |
| 3 | `(1,1)` | `96` |
| 4 | `(0,1)` | `128` |
| 5 | `(-1,1)` | `160` |
| 6 | `(-1,0)` | `192` |
| 7 | `(-1,-1)` | `224` |

Evidence: `Foundation_direction_table_init @ 0x0049F2F0` writes `0x0089F688=(0,-1)`, `0x0089F68C=(1,-1)`, `0x0089F690=(1,0)`, `0x0089F694=(1,1)`, `0x0089F698=(0,1)`, `0x0089F69C=(-1,1)`, `0x0089F6A0=(-1,0)`, `0x0089F6A4=(-1,-1)`.

### 3.2 8-bit and 16-bit facing relation

The useful Rust-facing conversion is:

```text
dir16 = facing8 << 8
facing8 = (((dir16 >> 7) + 1) >> 1) as byte
```

The second formula rounds a 16-bit value to the nearest high byte, not merely truncating. Current Rust has this exact conversion in `src/sim/miner/miner_dock_sequence.rs:59..63` for dock pivot state, but that pivot target itself is stale.

`ROT=` setup is separate from angle: `UnitClass::Constructor @ 0x007353C0` reads `type+0x71C`, and `FUN_004C9680 @ 0x004C9680` clamps `param_2 > 0x7E` to `0x7F`, then stores `(byte)param_2 << 8` into a FacingClass rate field. This is a per-frame turn amount, not a facing direction.

### 3.3 Facing helper arithmetic

The core helpers use signed 16-bit wraparound:

| Helper | Verified behavior | Evidence |
|---|---|---|
| `Facing__GetTurnDelta` | stores `(short)(target - current)` | decompile `Facing__GetTurnDelta` |
| `Facing__IsWithinROT` | compares `abs((short)(a-b)) <= abs(rot)` | decompile `Facing__IsWithinROT` |
| `Facing__ClampToROT` | if within ROT, snaps to target; else subtracts or adds `rot` along the signed shortest arc | decompile `Facing__ClampToROT` |

Active in YR: Yes. These helpers are called by `BulletClass__HomingTrack @ 0x005B20F0` and by other live facing/turn systems.

### 3.4 Movement vector to facing and facing to movement

`BulletClass__HomingTrack @ 0x005B20F0` computes yaw from live velocity/target vectors using `Math__atan2`, `Math__ftol`, and the same 16-bit signed-facing helpers. In the active DRAGON/AAHeatSeeker2 path, velocity is the source of rendered projectile orientation; no separate BulletClass visual-facing field was found in the homing slice.

For converting a facing back to movement, `TechnoClass::Fire_At @ 0x006FDD50` and `BulletClass__HomingTrack @ 0x005B20F0` use the standard sine/cosine pattern:

```text
dx = sin(angle) * speed
dy = -cos(angle) * speed
```

Current Rust `src/util/facing_table.rs:88` matches this 8-bit movement convention for aircraft/projectiles: `0` moves north/up, `64` moves east/right, `128` south/down, `192` west/left. Current Rust `src/util/fixed_math.rs:280` uses `atan2(dx, -dy)` for the inverse and tests the same cardinals.

### 3.5 8-way render/muzzle quantization

`TechnoClass::Fire_At @ 0x006FDD50` selects a directional muzzle anim only when `WeaponType.Anim.ActiveCount == 8` (`WeaponType+0x104 == 8`). The assembly at `0x006FF2E5..0x006FF30A` does:

```text
facing16 = this->vtable+0x308(...)
bucket = (((facing16 >> 12) + 1) >> 1) & 7
anim_index = (bucket + 1) & 7
```

The final `+1` is real in assembly (`INC ECX` after `AND ECX,0x7`), so the anim list is rotated one slot relative to the raw direction bucket. Current Rust `src/app_fire_effects.rs:55` implements the same rotated formula for `len == 8`.

### 3.6 32-way rotating SHP projectile mapping

The DRAGON draw-slot frame helper at `0x00468000` maps velocity to a 32-way SHP frame when `BulletType+0x2A1 == 0` (`Rotates=yes`):

```text
bam16 = ftol((atan2(-VelocityY, VelocityX) - pi/2) * (-32768/pi))
index = ((((uint16)bam16 >> 10) + 1) >> 1) & 0x1F
frame = DWORD_TABLE_007F4890[index]
```

`DWORD_TABLE_007F4890` is equivalent to `frame = (28 - index) & 31`. Active in YR: Yes for stock `[AAHeatSeeker2] Image=DRAGON`, `[DRAGON] Rotates=yes`.

Current Rust `src/app_fire_effects.rs:216` instead uses origin/destination cell delta and `frame = facing * frame_count / 256`. That is not the verified DRAGON formula and does not use live BulletClass velocity.

### 3.7 VXL facing surface

`VXL_GetFacingMatrix @ 0x007559B0` is a pure matrix-copy helper:

```text
copy 12 dwords from g_VXL_FacingMatrices + matrix_index * 0x30
```

This pass did not exhaust the callers that compute `matrix_index` from body/turret facing. Current Rust pre-renders body/composite and turret/barrel VXLs at 128 buckets (`step=2`) in `src/render/unit_atlas.rs:31..44` and truncates to even facing buckets in `canonical_unit_facing` / `canonical_turret_facing` at `src/render/unit_atlas.rs:1006..1016`. Treat exact gamemd VXL facing-bucket parity as not fully verified by this report.

### 3.8 Miner dock facing and the `0x4000` conflict

`UnitClass::Receive_Radio @ 0x00737430`, case `0x16`, does call active locomotor vtable `+0x4C` with `0x4000` when `unit+0x6AF == 0` and `RateTimer__Current() != 0x4000`.

However, concrete DriveLocomotion vtable `+0x4C` is `DriveLocomotionClass__Do_Turn @ 0x004B0EF0`, and the decompile is only:

```text
RateTimer__Set(&param_2)
```

Therefore this `0x4000` is a rate/timing synchronization value, not a facing target. The player-visible dump-phase body facing is not set by radio `0x16`; it is whatever the final drive path produced before `Power_Off`. Exit uses `BuildingClass::ReleaseDockedHarvester` -> `DriveLocomotionClass::Force_Track(0x47, ...)`, where `0x47` is a drive-track index, not raw facing byte `0x47`.

Active in YR: Yes for HARV and CMIN dock cycles. The older `miner/HARV_VS_CMIN_DUMP_FACING_COMPARISON_GHIDRA_REPORT.md` claim that `0x4000 = East` in radio `0x16` is stale; `miner/DOCK_ARRIVAL_PIVOT_SEQUENCE_GHIDRA_REPORT.md` contains the corrected interpretation.

## 4. INI Keys

| File | Section/key | Stock value | Binary effect | Active in YR |
|---|---|---:|---|---|
| `rulesmd.ini` `[CMIN] ROT` | `5` | parsed to `TechnoTypeClass+0x71C`; used by UnitClass FacingClass rate setup | Yes |
| `rulesmd.ini` `[HARV] ROT` | stock unit ROT value | same field path as CMIN | Yes |
| `rulesmd.ini` `[CMIN]/[HARV] Harvester` | `yes` | enables refinery dock/unload protocol; does not make radio `0x16` a facing setter | Yes |
| `artmd.ini` `[DRAGON] Rotates` | `yes` | stores inverted `BulletType+0x2A1 = 0`, enabling 32-way velocity frame mapping | Yes |
| weapon `Anim=` lists | count dependent | count `8` uses rotated 8-way formula; any other positive count falls back to first anim in verified Fire_At branch | Yes |

## 5. Integration Points

| Integration point | Verified behavior | Active in YR |
|---|---|---|
| Startup table init | `0x0049F2F0` writes N,NE,E,SE,S,SW,W,NW offsets before gameplay | Yes |
| `TechnoClass::Fire_At` | uses current facing for directional muzzle anim and projectile initial heading branches | Yes |
| `BulletClass::HomingTrack` | derives turn target from vector yaw and clamps with signed 16-bit Facing helpers | Yes for ROT projectiles |
| `BulletClass` draw frame helper | DRAGON rotating SHP uses velocity-derived 32-frame table | Yes |
| `UnitClass::Constructor` | `ROT=` becomes FacingClass rate, `(rot_byte << 8)` after clamp | Yes |
| `UnitClass::Receive_Radio` case `0x16` | sets locomotor rate/timer to `0x4000`, not facing | Yes |
| `DriveLocomotionClass::Process_Drive_Track` | converts track-point facing bytes via `(byte << 8)` before `FacingClass__UpdateFacing` | Yes |
| `VXL_GetFacingMatrix` | matrix helper indexes `g_VXL_FacingMatrices` by caller-supplied index | Yes, but caller bucketing deferred |

## 6. Current Rust Implementation Status

| Surface | Current status | Delta |
|---|---|---|
| `src/sim/movement/mod.rs:181` | public `facing_from_delta(dx,dy) -> u8` | cardinal contract matches the verified direction/facing order |
| `src/util/fixed_math.rs:280` | `atan2(dx, -dy)` inverse facing | matches cardinal order, but uses `f32`; acceptable for current 8-bit quantized sim only if all callers tolerate bucket-level parity |
| `src/util/fixed_math.rs:330` | `dir_to_cell_delta` rounds `(facing+16)/32 & 7` | matches direction table order |
| `src/util/facing_table.rs:88` | `dx=sin(facing)`, `dy=-cos(facing)` | matches verified facing-to-movement pattern |
| `src/app_fire_effects.rs:55` | 8-way muzzle anim formula includes final `+1` rotation | matches Fire_At assembly for 8 anim entries |
| `src/app_fire_effects.rs:216` | projectile frame from origin/destination delta and linear frame count | mismatch for DRAGON-style rotating SHP; should use velocity + `frame=(28-index)&31` for Rotates=yes 32-frame projectile art |
| `src/render/unit_atlas.rs:31..44`, `1006..1016` | VXL pre-rendered at 128 even-facing buckets | exact gamemd VXL caller bucket parity unchecked |
| `src/sim/miner/miner_dock_sequence.rs:47..48`, `713..759` | forces a smooth dock pivot to East using `0x4000` | mismatch; radio `0x16` is RateTimer/Do_Turn, not a facing write; dump-facing should not be forced to East from this evidence |

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| Direction index `0..7` order | verified | `0x0049F2F0`; `GDIRECTIONOFFSETS...` | none |
| 8-bit cardinal contract | verified | direction table + Rust-facing movement formulas + projectile/frame users | none for cardinal order |
| 16-bit signed Facing helpers | verified | `Facing__GetTurnDelta`, `Facing__IsWithinROT`, `Facing__ClampToROT` | none |
| `ROT=` clamp/shift | verified | `0x007353C0`, `0x004C9680` | none |
| Movement vector -> facing in homing | verified | `0x005B20F0`; AAHeatSeeker2 report | exact non-homing projectile variants out of scope |
| Facing -> movement vector | verified | `0x006FDD50`, `0x005B20F0` sine/cosine branches | none for core convention |
| 8-way muzzle anim index | verified | assembly `0x006FF2E5..0x006FF30A` | anim list semantic names out of scope |
| 32-way DRAGON frame | verified | assembly `0x00468000..0x0046805D`; table `0x007F4890` via prior report | none for DRAGON |
| VXL matrix index caller chain | touched-not-exhausted | `0x007559B0`; Rust scan | decompile draw-matrix callers that compute facing matrix index |
| Dock radio `0x16` | verified | `0x00737430`, `0x004B0EF0` | none for "not facing" finding |
| Exact dump-phase body facing value | deferred | `DOCK_ARRIVAL...`; static path-dependent | runtime trace per approach/path map |

## 8. Open Questions - Final State of the Investigation Log

- `[RESOLVED] OQ-1 - Is direction index order the same compass as 8-bit facing? -> Yes for adjacent movement: N,NE,E,SE,S,SW,W,NW corresponds to facing bytes 0,32,64,96,128,160,192,224.` (evidence: `0x0049F2F0`; Active in YR: Yes)
- `[RESOLVED] OQ-2 - Is 8-bit facing converted to 16-bit by high-byte placement? -> Yes for Rust-facing contract and current FacingClass surfaces: `facing8 << 8`; rounded reverse is `(((dir16>>7)+1)>>1)`.` (evidence: Rust scan plus binary `0x004C9680` rate-byte shift; Active in YR: Yes)
- `[RESOLVED] OQ-3 - How are 16-bit turns clamped? -> Signed short delta, absolute ROT compare, snap if within ROT, otherwise add/subtract ROT along shortest signed arc.` (evidence: `Facing__*` helpers; Active in YR: Yes)
- `[RESOLVED] OQ-4 - How does 8-way Fire_At anim selection round? -> `(((facing16>>12)+1)>>1)&7`, then rotated by `+1 mod 8`.` (evidence: `0x006FF2E5..0x006FF30A`; Active in YR: Yes)
- `[RESOLVED] OQ-5 - How does DRAGON rotating SHP frame mapping round? -> 32-way index is `((((uint16)bam>>10)+1)>>1)&31`; frame table equals `(28-index)&31`.` (evidence: `0x00468000..0x0046805D`, `DRAGON_BULLET_DRAW_SLOT_FRAME_MAPPING...`; Active in YR: Yes)
- `[RESOLVED] OQ-6 - Is refinery radio `0x16` a pivot to East? -> No. It calls DriveLocomotion vtable `+0x4C`, concretely `0x004B0EF0`, which calls `RateTimer__Set(&0x4000)`.` (evidence: `0x00737430`, `0x004B0EF0`; Active in YR: Yes)
- `[RESOLVED] OQ-7 - Is exit `0x47` a facing byte? -> No. `0x004B0C40` writes it to the drive-track index field and resets point index.` (evidence: `DriveLocomotionClass__Force_Track @ 0x004B0C40`; Active in YR: Yes)
- `[RESOLVED] OQ-8 - Does current Rust already encode the direction/facing cardinal order? -> Yes for `facing_from_delta`, `dir_to_cell_delta`, and `facing_to_movement`.` (evidence: `src/sim/movement/mod.rs:181`, `src/util/fixed_math.rs:280/330`, `src/util/facing_table.rs:88`)
- `[DEFERRED] OQ-9 - Exact VXL body/turret matrix bucket formula in gamemd callers.` (category: `bounded-cost-too-high`; reason: `0x007559B0` only copies caller-selected matrix; caller exhaustion is a separate render-slot investigation; next-step-if-pursued: trace Object/Techno draw-matrix callers into `VXL_GetFacingMatrix`)
- `[DEFERRED] OQ-10 - Exact body facing held during every miner dump scenario.` (category: `needs-runtime-debugger`; reason: dump-facing is path-dependent after final drive-in and static decompile proves absence of forced East pivot but not every path result; next-step-if-pursued: runtime trace HARV/CMIN approach path and body facing on pad)

## 9. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| Direction index `0..7` is N,NE,E,SE,S,SW,W,NW and maps to facing bytes `0,32,...224` | `0x0049F2F0` | none observed | `src/util/fixed_math.rs`, pathfinding direction enums | Preserve this as the single canonical direction/facing order | `direction_index_to_facing_byte_matches_gamemd_table` | Do not introduce keypad/order-by-screen-axis variants without explicit conversion |
| 16-bit FacingClass turn math uses signed short delta and ROT in high-byte units | `Facing__*`, `0x004C9680` | mostly implemented in `src/sim/movement/facing_class.rs` | `FacingClass`, turret/body rotation | Keep `rot_byte << 8`, clamp `>0x7E -> 0x7F`, shortest signed arc | `facing_class_signed_wrap_turns_across_zero` | Do not use unsigned absolute difference or long-way rotation |
| Fire_At 8-way anim selection rotates the rounded bucket by `+1 mod 8` | `0x006FF2E5..0x006FF30A` | none observed | `src/app_fire_effects.rs:55` | Preserve exact formula for `Anim` count 8 only | `weapon_muzzle_anim_8way_uses_rotated_gamemd_bucket` | Do not use raw `facing/32` list index |
| DRAGON Rotates=yes projectile SHP frame uses live velocity and table `(28-index)&31` | `0x00468000..0x0046805D` | mismatch | `src/app_fire_effects.rs:216`, projectile visual state | Route rotating SHP projectile frame from live velocity/facing and 32-way table | `dragon_projectile_frame_uses_velocity_32way_lookup` | Do not use origin-to-target cell delta after homing begins |
| Radio `0x16` dock sync does not set East facing | `0x00737430`, `0x004B0EF0` | mismatch and stale comments | `src/sim/miner/miner_dock_sequence.rs` | Remove forced East pivot as a claimed gamemd behavior; model rate/timing sync separately if needed | `miner_dock_radio_0x16_does_not_force_body_facing_east` | Do not treat `0x4000` as a facing in the DriveLocomotion `+0x4C` path |
| Refinery exit `0x47` is a drive-track index, not a raw facing | `0x004B0C40` | tests already mention this | `src/sim/movement/drive_track.rs`, miner exit sequence | Keep exit path track-index based | `turn_track_0x47_selects_raw_track_15_not_facing_0x47` | Do not store `entity.facing = 0x47` as the exit behavior |
| VXL matrix helper copies `g_VXL_FacingMatrices[index]` | `0x007559B0` | unchecked | `src/render/unit_atlas.rs`, `src/app_instances/units.rs` | Future render audit should prove caller bucket count/rounding before changing 128-bucket Rust atlas | `vxl_facing_bucket_count_matches_gamemd_draw_matrix_callers` | Do not infer VXL parity from DRAGON/SHP formulas |

## 10. Negative Facts / Do Not Do

- Do not call direction index `2` "north" or use keypad ordering; index `2` is East `(1,0)`.
- Do not collapse direction `8` into a normal facing/direction index; prior pathfinding reports show it is tube-specific in relevant path queues.
- Do not treat every literal `0x4000` as East. In `UnitClass::Receive_Radio` case `0x16`, it is a RateTimer value.
- Do not treat DriveLocomotion `Force_Track(0x47, ...)` as raw facing byte `0x47`.
- Do not use `facing * frame_count / 256` for DRAGON/Rotates=yes projectile SHP parity; the binary uses a rounded 32-way index and a rotated lookup table.
- Do not infer exact VXL draw-facing bucketing from the Rust atlas or from SHP projectile formulas; verify the VXL draw callers separately.

## 11. Remaining Uncertainty

- Exact VXL caller bucket formula remains a follow-up target. This report only proves the matrix-copy helper and current Rust surface.
- Exact dump-phase body facing for each refinery/map approach remains runtime/path dependent. Static evidence proves there is no forced East pivot in radio `0x16`.
- Some older docs conflict. This report treats `miner/DOCK_ARRIVAL_PIVOT_SEQUENCE_GHIDRA_REPORT.md` plus fresh decompilation as superseding `miner/HARV_VS_CMIN_DUMP_FACING_COMPARISON_GHIDRA_REPORT.md` on radio `0x16`.

## 12. Proposed Rust Test Names

- `direction_index_to_facing_byte_matches_gamemd_table`
- `facing_byte_to_direction_rounds_at_half_bucket`
- `facing_class_signed_wrap_turns_across_zero`
- `weapon_muzzle_anim_8way_uses_rotated_gamemd_bucket`
- `dragon_projectile_frame_uses_velocity_32way_lookup`
- `miner_dock_radio_0x16_does_not_force_body_facing_east`
- `refinery_exit_force_track_0x47_is_track_index_not_facing`
- `vxl_facing_bucket_count_matches_gamemd_draw_matrix_callers`

## Sources

- Ghidra MCP read-only decompile/assembly: `0x0049F2F0`, `0x006FDD50`, assembly `0x006FF2E5..0x006FF30A`, assembly `0x00468000..0x0046805D`, `0x005B20F0`, `Facing__GetTurnDelta`, `Facing__IsWithinROT`, `Facing__ClampToROT`, `0x007353C0`, `0x004C9680`, `0x00737430`, `0x004B0EF0`, `0x004B0F20`, `0x004B0C40`, `0x007559B0`.
- Prior reports: `GDIRECTIONOFFSETS_0089F688_BRIDGE_MARKER_PATH_GHIDRA_REPORT.md`, `DRAGON_BULLET_DRAW_SLOT_FRAME_MAPPING_GHIDRA_REPORT.md`, `DRAGON_RENDER_AND_GUARDWH_IMPACT_PRESENTATION_GHIDRA_REPORT.md`, `CMIN_RUNTIME_ROT_PARSER_OVERRIDE_GHIDRA_REPORT.md`, `miner/DOCK_ARRIVAL_PIVOT_SEQUENCE_GHIDRA_REPORT.md`, `miner/HARV_VS_CMIN_DUMP_FACING_COMPARISON_GHIDRA_REPORT.md`, `VXL_INTERPOLATED_FACING_AND_SLOPE_TRANSITION_GHIDRA_REPORT.md`.
- INI checked: `ini/rulesmd.ini`, `ini/artmd.ini`, `ini/rules.ini`, `ini/art.ini`.
- Rust scan: `src/sim/movement/mod.rs`, `src/util/fixed_math.rs`, `src/util/facing_table.rs`, `src/app_fire_effects.rs`, `src/render/unit_atlas.rs`, `src/app_instances/units.rs`, `src/sim/miner/miner_dock_sequence.rs`, `src/sim/movement/facing_class.rs`, `src/sim/movement/drive_track.rs`.
