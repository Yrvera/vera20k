# FLH Muzzle World Reference Point Trace

**Subagent slot:** 4
**Scenario:** A standard YR tank/techno fires a primary weapon with nonzero FLH while facing east on flat terrain.
**Concrete fixture:** `[APOC]` primary `[120mmx]`, image `[MTNK]`, `PrimaryFireFLH=190,25,120`, `Burst=2`, facing east (`u8=64`, DirStruct word `0x4000`), flat terrain (`z=0`).
**Scope limit:** FLH muzzle/world reference point, 32-way facing quantization, and burst lateral sign behavior against current Rust fire-origin/muzzle/sound handling.

## Verdict Tally

PASS: 1 | FAIL: 5 | UNCHECKED: 2 | NOT-IMPLEMENTED: 1

## Pipeline

`AttackTarget fire tick` -> `weapon/art data` -> `SimFireEvent` -> `FLH transform` -> `muzzle flash/report sound` -> `projectile visual start` -> `screen/audio result`

## Stage 1 - Scenario Data Selection

**Inputs:** `[APOC] Image=MTNK`, `Primary=120mmx`; `[MTNK] PrimaryFireFLH=190,25,120`; `[120mmx] Projectile=Cannon`, `Speed=40`, `Report=ApocalypseAttackGround`, `Anim=APMUZZLE`, `Burst=2`.

**Rust:** `ArtEntry` stores primary/secondary/elite FLH at `src/rules/art_data.rs:66`, parsed at `src/rules/art_data.rs:294`, resolved by `resolve_metadata_entry` at `src/rules/art_data.rs:542`. `resolve_fire_origin_from_art` selects primary FLH through `resolve_flh` at `src/app_fire_effects.rs:74`.

**gamemd:** The FLH report verifies normal weapon FLH slots are read by `TechnoClass::GetFLH @ 0x006F3AD0` through `TechnoClass::GetWeapon @ 0x0070E140`; active in standard YR.

**Concrete values:** both sides use primary FLH `(190,25,120)` for this fixture.

**Verdict:** PASS.

## Stage 2 - Fire Event Snapshot

**Rust:** `SimFireEvent` carries attacker id/type, weapon slot/id, facing, veterancy, target, report sound, and garrison fields only (`src/sim/world/mod.rs:193`). It is emitted at the fire tick in `src/sim/combat/mod.rs:1974`.

**gamemd:** `TechnoClass::Fire_At @ 0x006FDD50` calls virtual `GetFLH` before bullet allocation/init and before muzzle sound/anim effect creation; active in standard YR per `FLH_TURRET_AND_VISUAL_OFFSETS_GHIDRA_REPORT.md`.

**Mismatch:** Rust does not snapshot the computed world/lepton source coordinate or the current burst index needed by `GetFLH`. It only snapshots facing and later recomputes a presentation-only screen offset.

**Verdict:** FAIL.

## Stage 3 - 32-Way Facing Quantization

**Rust:** `flh_to_screen_offset_32way` computes `facing_16 = facing << 8`, `bucket = ((((facing_16 >> 10) + 1) >> 1) & 0x1f) - 8`, then uses `quantized_facing = (bucket + 8) * 8` (`src/util/flh_transform.rs:78`).

**Concrete Rust value:** facing `64` -> `facing_16=0x4000`, `bucket=0`, `quantized_facing=64` (east).

**gamemd:** Existing binary report verifies the same 32-way bucket family in `TechnoClass::GetFLH @ 0x006F3AD0`, with active standard YR use.

**Caveat:** This stage only confirms the east-facing bucket. It does not prove Rust's later screen-space transform equals gamemd's world matrix output.

**Verdict:** UNCHECKED. East bucket selection matches conceptually, but this trace did not reduce Rust's screen-space facing byte and gamemd's matrix angle to the same numeric output representation.

## Stage 4 - FLH World Source Coordinate

**Rust:** `resolve_fire_origin_from_art` converts FLH directly to screen pixels and returns `FireOrigin { screen_x, screen_y, rx: position.rx, ry: position.ry, z: position.z }` (`src/app_fire_effects.rs:82` and `src/app_fire_effects.rs:88`). The world reference cell/z stays the attacker position.

**Concrete Rust screen offset for APOC east:** FLH `(190,25,120)`, facing `64` -> quantized facing `64`; local screen delta `(19.3359375, -4.40234375)`, `AdjustForZ(120)=17`. World fields remain unchanged: `rx=attacker.rx`, `ry=attacker.ry`, `z=attacker.z`.

**gamemd:** `TechnoClass::GetFLH` returns a world `CoordStruct` in leptons: FLH transformed by the 32-way voxel/matrix path, rounded by `Math__ftol`, then added to `GetRenderCoords`. The returned world coordinate is consumed by bullets, muzzle anims, sounds, lasers, waves, and special effects. Active in standard YR.

**Mismatch:** For nonzero FLH, gamemd's source world coordinate is not the attacker's unchanged cell/z. Rust never computes that world source, so projectile/sound/muzzle downstream cannot be numerically equal.

**Verdict:** FAIL.

## Stage 5 - Burst Lateral Sign

**Rust:** Burst state is tracked as `burst_remaining`/`burst_delay_ticks` (`src/sim/combat/mod.rs:1999`), but `SimFireEvent` has no burst index field (`src/sim/world/mod.rs:193`). `resolve_fire_origin_from_art` always passes `flh.lateral` unchanged (`src/app_fire_effects.rs:82`).

**Concrete Rust result:** APOC burst shot 1 and burst shot 2 both use lateral `+25` and the same screen offset.

**gamemd:** `TechnoClass::GetFLH` reads `CurrentBurstIndex` (`+0x3B8`) and flips the second translate component: odd current burst uses positive lateral, even current burst uses negative lateral. Active in standard YR.

**Player-visible difference:** A two-barrel tank's muzzle flash/report/projectile source does not alternate sides in Rust; gamemd alternates the barrel-side reference point.

**Verdict:** FAIL.

## Stage 6 - Muzzle Flash Origin

**Rust:** Non-garrison muzzle flashes use `origin.screen_x/screen_y` and `origin.rx/ry/z` from the presentation `FireOrigin` (`src/app_fire_effects.rs:167`). The SHP anim is selected from `weapon.anim` at `src/app_fire_effects.rs:159`.

**gamemd:** The same `GetFLH` world source coordinate feeds muzzle `AnimClass` construction in `TechnoClass::Fire_At`; active in standard YR.

**Mismatch:** Rust's visible flash may be approximately near the barrel, but it is driven by a screen-only helper and cannot share the gamemd world source or burst-side alternation.

**Verdict:** FAIL.

## Stage 7 - Report Sound Origin

**Rust:** Non-garrison report sound uses `GameSoundEvent::WeaponFired { screen_pos: Some((origin.screen_x, origin.screen_y)) }` (`src/app_fire_effects.rs:150`). Spatial volume then uses the screen position at `src/app_building_anim.rs:645`.

**gamemd:** The FLH report verifies `TechnoClass::Fire_At` uses the same computed source coordinate for muzzle report sound (`VocClass__PlayAt`) as for projectile/muzzle origin; active in standard YR.

**Mismatch:** Rust spatial audio is tied to a presentation screen offset, not a world/lepton source coordinate projected through the same contract as bullets and muzzle effects. With the burst-side bug, both burst report sounds also stay on the same side.

**Verdict:** FAIL.

## Stage 8 - Projectile Start / Direction / Duration

**Rust:** `build_projectile_visuals` resolves `origin`, but stores `start_rx/start_ry` from `origin.rx/origin.ry`, which remain the attacker cell (`src/app_fire_effects.rs:281`). Direction and duration use cell deltas from `origin.rx/ry` to target (`src/app_fire_effects.rs:216`, `src/app_fire_effects.rs:226`).

**gamemd:** Standard YR fire allocates a real `BulletClass` from `TechnoClass::Fire_At` and launches it from the `GetFLH` world source. The GGI lifecycle report confirms this Fire_At -> BulletClass path is live in stock YR; the FLH report confirms the source coordinate contract.

**Mismatch:** Rust has a render-only projectile visual for this path, not a sim-owned bullet source coordinate. For this FLH scenario, projectile start/direction/duration are cell-center approximations rather than FLH world-source outputs.

**Verdict:** NOT-IMPLEMENTED.

## Stage 9 - Timing / Ordering

**Rust:** `sim.fire_events` are drained after `advance_tick` (`src/app_sim_tick.rs:314`), then non-garrison effects are spawned later from those drained events (`src/app_fire_effects.rs:295`). Muzzle animation ticking occurs in app frame time (`src/app_fire_effects.rs:318`).

**gamemd:** Fire_At computes source, creates sound/anim/projectile work in one fire path. Existing docs verify ordering at the source-coordinate level, but this trace did not compute exact app-frame presentation latency against gamemd.

**Verdict:** UNCHECKED.

## Top Player-Visible Findings

1. **Stage 4 - source coordinate:** projectile/muzzle source uses attacker cell/z instead of gamemd FLH world `CoordStruct`; `src/app_fire_effects.rs:88`; gamemd `TechnoClass::GetFLH @ 0x006F3AD0`, active YR.
2. **Stage 5 - burst lateral sign:** two-barrel primary burst does not alternate side; `src/sim/world/mod.rs:193` and `src/app_fire_effects.rs:82`; gamemd reads `CurrentBurstIndex +0x3B8` in `GetFLH`.
3. **Stage 8 - projectile start:** projectile visual starts/directions from cell deltas, not FLH source; `src/app_fire_effects.rs:216` and `src/app_fire_effects.rs:281`; gamemd Fire_At launches BulletClass from `GetFLH` source.
4. **Stage 7 - report sound:** weapon report spatial origin is a screen-only FLH approximation; `src/app_fire_effects.rs:150`; gamemd `VocClass__PlayAt` uses the Fire_At source coordinate.
5. **Stage 6 - muzzle flash:** flash placement does not share a world source or burst-side alternation; `src/app_fire_effects.rs:167`; gamemd muzzle `AnimClass` is constructed from the same FLH source.

## Adjacent Findings

- Building `PrimaryFirePixelOffset` / `SecondaryFirePixelOffset`, garrison fire ports, and building voxel turret origins are verified in the FLH report but intentionally not traced here.
- `WeaponNFLH` and `AlternateFLH0..4` are adjacent helper gaps for multi-weapon/open-topped-style sources, but this scenario used one primary tank weapon only.
- Exact final pixel equality for Rust's `flh_to_screen_offset_32way` versus gamemd's world matrix plus projection remains a separate numeric audit.

## Sources

- `C:/Users/enok/Documents/ra2-rust-game-docs/FLH_TURRET_AND_VISUAL_OFFSETS_GHIDRA_REPORT.md`
- `C:/Users/enok/Documents/ra2-rust-game-docs/GGI_MISSILELAUNCHER_AAHEATSEEKER2_PROJECTILE_LIFECYCLE_GHIDRA_REPORT.md`
- `ini/rulesmd.ini` (`[APOC]`, `[120mmx]`)
- `ini/artmd.ini` (`[MTNK]`)
- Current Rust scan: `src/app_fire_effects.rs`, `src/util/flh_transform.rs`, `src/sim/combat/mod.rs`, `src/sim/world/mod.rs`, `src/rules/art_data.rs`, `src/app_sim_tick.rs`, `src/app_building_anim.rs`

## Status

COMPLETE
