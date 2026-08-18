# AnimClass Bouncer Impact Gates - Ghidra Report

**Date:** 2026-05-28  
**Investigation mode:** exhaustive-slice  
**Target:** AnimClass bouncer/meteor `BounceAnim=` + `ExpireAnim=` impact gates and damage side effects.  
**Primary addresses:** `AnimClass::Constructor @ 0x00421EA0`, `AnimClass::ProcessBounceResult @ 0x00423930`, `AnimClass::AI @ 0x00423AC0`, `AnimClass::Destroy @ 0x004255B0`, `AnimTypeClass::ReadINI @ 0x00427D00`, `BounceClass::Update @ 0x00439B00`, `Apply_area_damage @ 0x00489280`.

Working notes:
- `Target question`: What exact active-YR bouncer/meteor impact path gates `Bouncer`, `IsMeteor`, `BounceAnim`, `ExpireAnim`, `Damage`, `DamageRadius`, and `Warhead`, what constructor rows are emitted, and what side effects must Rust preserve?
- `Non-goals`: Do not investigate `TrailerAnim`, global constructor taxonomy, full water-splash visuals, full `Apply_area_damage` internals, or non-AnimClass `VoxelAnimClass` bouncers.
- `Evidence needed to mark COMPLETE`: Decompile plus assembly-context evidence for key reads/branches/constructor args, INI/default source plus binary reader address for parser fields, active standard-YR retail examples, current Rust surface scan, and doc-staleness replacement wording.
- `Stop conditions`: Stop when `BounceAnim`/`ExpireAnim` row ownership, return-code gates, accepted-water/terrain gates, damage gating, normal-destroy negative fact, Rust handoff, and stale-doc wording are resolved or explicitly deferred.

## Summary

`Bouncer=yes` and `IsMeteor=yes` are active conditional `AnimTypeClass` flags in Yuri's Revenge. The `AnimClass` constructor turns either flag into the per-instance bouncer byte at `AnimClass+0x194`; `AnimClass::AI` checks that byte and calls vtable `+0x1E8`, which is `AnimClass::ProcessBounceResult`.

The impact path is split:

- `AnimClass::ProcessBounceResult @ 0x00423930` calls `BounceClass::Update @ 0x00439B00`. Return `1` means bounce this tick; it may spawn `BounceAnim=` from `AnimType+0x300` and then performs direct same-cell object damage using Manhattan XY distance against `DamageRadius`.
- `AnimClass::AI @ 0x00423AC0` receives return `1` or `2`. If the impact is accepted by the terrain/water gate and `ExpireAnim=` at `AnimType+0x304` is non-null, it may spawn `ExpireAnim=`, then applies area damage and debris-smoke helper side effects, and later destroys the parent anim.
- Normal `AnimClass::Destroy @ 0x004255B0` does not spawn `ExpireAnim=`.

Active in YR: Conditional. Retail `ini/artmd.ini` contains active bouncer and meteor sections with `ExpireAnim=`, `Damage=`, `DamageRadius=`, `Warhead=`, and either `Bouncer=yes` or `IsMeteor=true`; stock retail `art.ini`/`artmd.ini` contain no `BounceAnim=` assignment, so the `BounceAnim` branch is active engine code for mods/custom data but not confirmed by stock retail INI rows.

## Verified Findings

### Parser fields and defaults

Active in YR: Yes for parsing; conditional for runtime use.

`AnimTypeClass::ReadINI @ 0x00427D00` reads the relevant keys directly into these fields:

| Key | Field | Default behavior | Evidence |
|---|---:|---|---|
| `Damage=` | `AnimType+0x2A8` double | keeps existing/default double when absent | decompile `0x00427D00`; read at `0x00423E8B` and `0x00423A46` |
| `BounceAnim=` | `AnimType+0x300` pointer | null when absent or lookup rejected | read/store range `0x00428415..0x004284B5`; spawn read `0x00423981..0x004239CE` |
| `ExpireAnim=` | `AnimType+0x304` pointer | null when absent or lookup rejected | read/store range `0x004284B5..0x00428573`; AI read `0x00423DED` |
| `DamageRadius=` | `AnimType+0x334` int | keeps prior/default int when absent | assembly context `0x00428651..0x00428682`; ProcessBounceResult compare `0x00423A3C..0x00423A44` |
| `Warhead=` | `AnimType+0x330` pointer | keeps prior/default pointer unless key resolves | assembly context `0x00428665..0x0042869C`; damage reads `0x00423A59`, `0x00423E85`, `0x00423EC0` |
| `IsMeteor=` | `AnimType+0x356` bool | false unless read true | decompile `0x00427D00`; ProcessBounceResult read `0x00423948..0x00423962`; AI water branch read `0x00423CE0` |
| `Bouncer=` | `AnimType+0x35A` bool | false unless read true | assembly context `0x0042869E..0x004286B8`; constructor gate `0x00421EA0` decompile |

Retail activation evidence: `ini/artmd.ini:14986..14999` (`DBRIS1LG`) has `ExpireAnim=TWLT036`, `Damage=20`, `DamageRadius=80`, `Warhead=HE`, `Bouncer=yes`; `ini/artmd.ini:19065..19069` (`METLARGE`) and `19086..19090` (`METSMALL`) have `IsMeteor=true`; `ini/artmd.ini:19108..19119` (`METDEBRI`) has bouncer impact damage data. Active in YR: Conditional on these anim types being constructed by gameplay.

### Instance activation gate

Active in YR: Conditional.

`AnimClass::Constructor @ 0x00421EA0` checks `AnimType+0x35A Bouncer` and `AnimType+0x356 IsMeteor`. If either is true, it sets instance byte `AnimClass+0x194` and initializes embedded `BounceClass` state through `BounceClass::Init`; if both are false, it follows normal object reveal without bouncer initialization. `AnimClass::AI @ 0x00423C24..0x00423C44` reads byte `AnimClass+0x194`; when set, it calls vtable `+0x1E8` and only continues the impact branch for return `1` or `2`.

Load-bearing evidence: decompile of constructor `0x00421EA0`; AI assembly context `0x00423C24` shows `MOV AL,[ESI+0x194]`, `CALL [EAX+0x1E8]`, `CMP EAX,0x2`, `CMP EAX,0x1`.

### `ProcessBounceResult` return semantics

Active in YR: Conditional.

`AnimClass::ProcessBounceResult @ 0x00423930` calls `BounceClass::Update @ 0x00439B00` and preserves its return code:

- Return `0`: no bounce/stop side effect in this slice; function updates current coords through vtable `+0x1B4` and returns `0`.
- Return `1`: bounce this tick. The function may spawn `BounceAnim=` and runs direct same-cell object damage.
- Return `2`: stop/final impact. The function calls vtable `+0xF8` (`AnimClass::Destroy`) immediately at `0x00423971..0x00423976`, then still returns `2`; the caller `AnimClass::AI` continues into its accepted-impact branch before its final destroy call.

Evidence: decompile of `0x00423930`; assembly context `0x00423965..0x00423981` and `0x00423971..0x00423976`. Active in YR: Conditional on `AnimClass+0x194`.

### `BounceAnim=` row

Active in YR: Conditional; standard retail has no stock `BounceAnim=` key found in `ini/art.ini` or `ini/artmd.ini`, but the parser and branch are live for custom/modded data.

On return `1`, `ProcessBounceResult` reads `AnimType+0x300`; if non-null and allocation of `0x1C8` succeeds, it constructs:

`AnimClass(type->BounceAnim, this->GetCoords(...), delay=0, loop=1, drawFlags=0x600, zAdjust=0, reverse=0)`.

Evidence: assembly context at `0x00423981` reads `[EBP+0xC8]` then `[EAX+0x300]`; `0x004239A7..0x004239CE` pushes `0`, `0`, `0x600`, `1`, `0`, coords, type, then calls `0x00421EA0`. This proves `BounceAnim` uses `0x600`, not `0x2600`.

### Direct bounce damage on return `1`

Active in YR: Conditional.

After optional `BounceAnim`, return `1` scans objects in the current cell list from `CellClass+0xE4`. For each object, it computes `abs(dx) + abs(dy)` using integer coordinate components and compares that sum to `AnimType+0x334 DamageRadius`. The comparison is inclusive: if `distance <= DamageRadius`, it converts `Damage` from `AnimType+0x2A8` with `Math::ftol`, reads `Warhead` from `AnimType+0x330`, calls `Tactical::AdjustForZ(warhead,0,0,0,0)`, and invokes the object's vtable `+0x16C` damage receiver.

Evidence: decompile `0x004239D3..0x00423A83`; assembly context `0x00423A3C..0x00423A44` shows compare against `[EBX+0x334]` and `JG 0x00423A83`, so equality is accepted. `0x00423A46..0x00423A7D` reads `Damage` and `Warhead` and calls object vtable `+0x16C`.

### `ExpireAnim=` accepted-impact gate

Active in YR: Conditional.

`AnimClass::AI` only considers this branch after bouncer vtable `+0x1E8` returns `1` or `2`. It gets the current cell and computes:

- `non_water = CellClass+0xEC != 2`.
- `above_or_at_ground = AnimZ >= CellGroundHeight + DAT_0089A1B4`.

The accepted-impact branch runs when `non_water || above_or_at_ground`. If the cell is water (`+0xEC == 2`) and the anim is below the ground gate, AI takes the water/splash branch and does not construct `ExpireAnim=`.

Evidence: decompile `0x00423C4A..0x00423CCF`; assembly context `0x00423CC1` shows `TEST BL,BL; JZ 0x00423DE7; TEST AL,AL; JNZ 0x00423DE7`, routing accepted cases to the ExpireAnim branch. Active in YR: Conditional on bouncer/meteor return `1` or `2`.

### `ExpireAnim=` row and side effects

Active in YR: Conditional.

On an accepted impact, AI reads `AnimType+0x304 ExpireAnim`. If it is null, it jumps to `0x00423EFD` and skips the `ExpireAnim` constructor, the `Apply_area_damage` call, and the `FUN_0048A620` helper. If it is non-null, allocation is attempted. Allocation success only gates the visual row; it does not gate damage side effects.

If allocation succeeds, AI converts the embedded `BounceClass` float position at `AnimClass+0x140` through three `Math::ftol` calls and constructs:

`AnimClass(type->ExpireAnim, impact_coords, delay=0, loop=1, drawFlags=0x2600, zAdjust=-30, reverse=0)`.

Then AI reads `Damage` (`+0x2A8`) and `Warhead` (`+0x330`) and calls:

- `Apply_area_damage @ 0x00489280` with converted impact coords, integer damage, warhead, owner/object context from this branch, and the visible push sequence shown at `0x00423E75..0x00423EAB`.
- `FUN_0048A620 @ 0x0048A620` after another damage/warhead/coord setup at `0x00423EB0..0x00423EF8`.

Evidence: assembly context `0x00423DE7` reads `[EDX+0x304]` and `JZ 0x00423EFD`; `0x00423E51..0x00423E70` pushes `reverse=0`, `zAdjust=-30`, `drawFlags=0x2600`, `loop=1`, `delay=0`, coords, type, then calls constructor; `0x00423E75..0x00423EAB` calls `Apply_area_damage`; `0x00423EB0..0x00423EF8` calls `FUN_0048A620`. Allocation failure jumps to `0x00423E75`, so damage still runs when the type field is non-null.

### Same-tick ordering

Active in YR: Conditional.

When `BounceClass::Update` returns `1` on an accepted impact and both refs are non-null, the native order in one parent AI visit is:

1. `ProcessBounceResult` constructs `BounceAnim=` (`0x004239A7..0x004239CE`).
2. `ProcessBounceResult` applies direct same-cell bounce damage (`0x004239D3..0x00423A83`).
3. Control returns to `AnimClass::AI`.
4. AI constructs `ExpireAnim=` (`0x00423DE7..0x00423E70`).
5. AI applies area damage and helper side effects (`0x00423E75..0x00423EF8`).
6. AI later calls parent destroy (`vtable +0xF8`) after accepted-impact handling.

Evidence: decompile and assembly contexts for `0x00423930` and `0x00423AC0`. Active in YR: Conditional; stock standard-YR same-tick double-row from stock INI is unconfirmed because no stock `BounceAnim=` key was found.

### Normal destroy negative fact

Active in YR: Yes.

`AnimClass::Destroy @ 0x004255B0` detaches from owner, calls `SetOwnerObject(NULL)`, releases sound, optionally plays `StopSound=` from `AnimType+0x2FC` at sparkle coords, then calls `ObjectClass::UnInit`. It does not read `AnimType+0x304`, does not test `ExpireAnim=`, and does not call `AnimClass::Constructor`.

Evidence: decompile of `0x004255B0`; assembly context `0x004255B0..0x0042561F` shows owner detach, `0x00424B50`, sound release, read `[ESI+0xC8]`, read `[EAX+0x2FC]`, `VocClass__PlayAt`, then `ObjectClass__UnInit`. Active in YR: Yes for AnimClass cleanup.

## Current Rust Delta

`src/rules/art_data.rs` currently parses `bounce_anim`, `expire_anim`, `trailer_anim`, and `trailer_seperation` into `AnimTypeRuntimeConfig` (`src/rules/art_data.rs:180`, `188..190`, `286..288`). It does not expose the full bouncer activation and damage surface (`Bouncer`, `IsMeteor`, `Damage`, `DamageRadius`, `Warhead`) in that runtime metadata.

`src/sim/components.rs` has `AnimClassSpawnDescriptor` with `type_name`, coords, `delay`, `loop_count`, `draw_flags`, `z_adjust`, and `reverse` (`src/sim/components.rs:769..811`) and `WorldEffect::from_anim_spawn` preserves the descriptor (`src/sim/components.rs:844..867`). No current Rust bouncer/meteor runtime was found that calls a BounceClass-like update, emits `BounceAnim=`, emits `ExpireAnim=`, applies bouncer damage, or preserves same-tick row ordering.

Active in YR relevance: Conditional but player-visible for debris/meteor anims defined by retail art data and for mods using `BounceAnim=`.

## Implementation Handoff

| Verified behavior | Rust delta | Affected surface | Acceptance scenario | Proposed test name | Risk |
|---|---|---|---|---|---|
| `Bouncer=yes` or `IsMeteor=true` sets a per-instance bouncer path; AI calls ProcessBounceResult only for that byte. | Add full runtime metadata and bouncer/meteor anim lifecycle, not just parsed child refs. | `src/rules/art_data.rs`, future generic `AnimClass` runtime, `src/sim/components.rs` spawn rows. | Construct a test anim with `Bouncer=yes`, `ExpireAnim=TWLT036`, `Damage=20`, `DamageRadius=80`, `Warhead=HE`; assert the parent enters bouncer update while a non-bouncer with the same `ExpireAnim` does not. | `anim_bouncer_flag_enables_impact_runtime_only_for_bouncer_or_meteor` | High: applying ExpireAnim to all anim expiry would create false visuals and damage. |
| Return `1` emits optional `BounceAnim` first, then direct same-cell inclusive-radius damage; accepted AI impact can later emit `ExpireAnim` and area damage. | Model two distinct damage/visual phases and preserve order within the same parent AI visit. | Future bouncer runtime; damage system bridge; renderer-visible effect queue. | Force bounce return `1` with both child refs and one object at exactly `DamageRadius`; assert row order `BounceAnim` before `ExpireAnim` and object at equality receives the direct bounce damage. | `anim_bouncer_return_one_orders_bounce_row_damage_then_expire_row` | High: collapsing to one impact event loses ordering, inclusive radius, and duplicate damage surfaces. |
| `ExpireAnim != null` gates AI area damage/smoke helper; allocation failure only skips the visual row. | Tie AI impact side effects to the parsed `ExpireAnim` ref, not renderer allocation success; if Rust allocation cannot fail, still preserve null-ref gating. | Future bouncer runtime; area-damage call surface; effect allocation path. | Accepted impact with no `ExpireAnim` but nonzero `Damage`/`Warhead` produces no AI area damage; same anim with `ExpireAnim` produces area damage even if visual creation is disabled in a test harness. | `anim_bouncer_expireanim_ref_gates_ai_area_damage_not_visual_allocation` | High: treating `Damage=` alone as active changes stock debris/meteor damage; tying damage to visual allocation makes render failure affect sim. |
| Water below the ground gate rejects the `ExpireAnim` branch and takes a separate water/splash path. | Implement accepted-impact gate before spawning ExpireAnim or applying its AI area damage. | Future bouncer runtime; terrain/water query; effect queue. | Force accepted ground and rejected water impacts for the same anim; assert ground emits `ExpireAnim`/area damage while below-water path does not emit `ExpireAnim`. | `anim_bouncer_water_below_ground_skips_expireanim_branch` | Medium: water impacts are less common but visibly wrong when meteors/debris land in water. |
| Normal `AnimClass::Destroy` does not spawn `ExpireAnim`. | Keep generic expiry/destruction separate from bouncer accepted-impact branch. | `WorldEffect::tick_with_start_sound`, future `AnimClass::Destroy` equivalent. | Let a non-bouncer anim with `ExpireAnim=TWLT036` finish normally; assert no `TWLT036` child is spawned by destroy/expiry. | `anim_destroy_does_not_spawn_expireanim` | High: attractive shortcut would over-spawn impact explosions. |

## Negative Facts / Do Not Do

- Do not set bouncer `BounceAnim` `drawFlags` to `0x2600`; verified constructor pushes `0x600` at `0x004239AB`.
- Do not spawn `ExpireAnim=` from `AnimClass::Destroy`; destroy reads `StopSound +0x2FC`, not `ExpireAnim +0x304`.
- Do not make `Damage=` alone activate AI impact area damage; the AI damage block is skipped when `ExpireAnim +0x304` is null.
- Do not tie AI impact damage to successful visual allocation; allocation failure jumps to the damage block at `0x00423E75`.
- Do not merge `BounceAnim=` and `ExpireAnim=` into one "impact animation"; they are different fields, different functions, different flags/z-adjusts, and different order.

## Remaining Uncertainty

- Exact water/splash branch assets and order at `0x00423CD5..0x00423DE2` are out of scope and remain deferred.
- Exact full `Apply_area_damage @ 0x00489280` downstream side effects are out of scope here; this report only proves the bouncer branch call and its gate.
- Stock retail same-tick `BounceAnim` plus `ExpireAnim` emission was not confirmed because stock `ini/art.ini`/`ini/artmd.ini` have no `BounceAnim=` assignment.
- Full gameplay caller taxonomy for every retail bouncer/meteor anim construction is not repeated here; prior constructor-taxonomy work owns that broader map.

## Stale Docs / Replacement Wording

- `docs/research/ANIMCLASS_SPAWN_PATHS_GHIDRA_REPORT.md` has a stale row in the drawFlags table: `Bouncer BounceAnim (on impact) | 0x2600`. Replace it with:
  - `Bouncer BounceAnim (on bounce return 1) | 0x600 | center sprite | ProcessBounceResult reads AnimType+0x300 and pushes drawFlags 0x600 at 0x004239AB.`
  - `Bouncer ExpireAnim (accepted impact) | 0x2600 | center + Z-buffer | AnimClass::AI reads AnimType+0x304 and pushes drawFlags 0x2600 at 0x00423E55, zAdjust -30 at 0x00423E53.`
- `docs/research/ANIMCLASS_BOUNCER_METEOR_EXPIREANIM_IMPACT_SPAWNS_GHIDRA_REPORT.md` says allocation success is part of the `ExpireAnim` impact branch gate. Refine replacement wording:
  - `ExpireAnim +0x304 non-null gates the AI impact constructor and subsequent Apply_area_damage/FUN_0048A620 block. Allocation success gates only the ExpireAnim visual constructor; allocation failure jumps to 0x00423E75, so AI damage/helper side effects still run when ExpireAnim is non-null.`

## Sources

- Read-only Ghidra decompile: `0x00421EA0`, `0x00423930`, `0x00423AC0`, `0x004255B0`, `0x00427D00`, `0x00439B00`, `0x00489280`.
- Read-only Ghidra assembly context: `0x00423981`, `0x00423A44`, `0x00423C24`, `0x00423CC1`, `0x00423DE7`, `0x00423E51`, `0x00423E70`, `0x00423E75`, `0x00423EAB`, `0x00428651`, `0x004255B0`.
- Retail INI: `ini/artmd.ini` bouncer/meteor examples listed above; `rg "^BounceAnim=" ini/art.ini ini/artmd.ini` returned no stock rows.
- Rust surface scan: `src/rules/art_data.rs`, `src/sim/components.rs`.
