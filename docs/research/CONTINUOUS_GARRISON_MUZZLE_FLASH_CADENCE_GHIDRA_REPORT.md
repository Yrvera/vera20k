# Continuous Garrison Muzzle Flash Cadence - Ghidra Research Report

**Address(es):** `BuildingClass::Update @ 0x0043FB20`, chrono visual sub-branch `0x004403D4..0x0044055D`; contrast path `TechnoClass::Fire_At @ 0x006FDD50`.
**Investigation Mode:** exhaustive-slice
**Claimed Scope:** Whether `BuildingClass::Update` spawns a continuous/ambient garrison muzzle flash; if not, what the verified branch actually does, including cadence, frame formula, port bounds, coordinate source, anim source, and gates.
**Non-Scope:** Weapon selection, owner timing, kill credit, target acquisition, and full chrono teleport/temporal lifecycle.
**Confidence:** High
**Active in YR:** Conditional. The verified `BuildingClass::Update` branch is active for buildings whose Techno warp flags are set (`+0x270` or `+0x271`), not for ordinary occupied garrison firing.

## 0. Investigation Contract

Target question: Does `BuildingClass::Update @ 0x0043FB20` create a separate continuous garrison muzzle-flash visual, and if so what are the exact cadence, gates, coordinates, and anim source?

Non-goals: Do not re-study garrison weapon selection, current fire index ownership, owner transfer timing, or the full chrono visual pipeline. Use `TechnoClass::Fire_At` only to distinguish actual shot-triggered `OccupantAnim` from the `BuildingClass::Update` branch.

Evidence needed to mark COMPLETE: decompile plus assembly/disassembly for the `BuildingClass::Update` branch; vtable slot proof for the branch gates; INI/default proof for `RulesClass+0x344`; contrast evidence from `Fire_At` for `WeaponType+0x110`; current Rust surface scan; stale-doc replacement wording; implementation handoff with at least one acceptance scenario.

Stop conditions: Stop once all scoped branch constants, formulae, bounds, sources, and gates are verified, and a final cold pass over the branch adds no new open questions. Do not expand into weapon choice or chrono lifecycle after identifying the branch gates.

## 1. Overview

`BuildingClass::Update` does not contain a normal occupied-garrison ambient muzzle-flash path. The branch previously described as continuous garrison fire is a chrono/temporal visual early-return path reached only when `TechnoClass::IsWarpingOut` or `TechnoClass::IsBeingWarped` returns nonzero.

That branch reuses `BuildingTypeClass+0x1580` and `+0x1588` as a count and per-slot isometric pixel offset list, but its anim source is `RulesClass+0x344`, which maps to `[General] ChronoSparkle1`, not to weapon `OccupantAnim` or any garrison muzzle-flash key. Actual shot-triggered garrison flashes remain in `TechnoClass::Fire_At`, where an occupied building replaces normal weapon `Anim=` with `WeaponType+0x110`.

## 2. Class Layout / Key Offsets

| Offset | Owner | Meaning in this slice | Evidence | Active in YR |
|---|---|---|---|---|
| `+0x270` | `TechnoClass` | warp-out flag returned by vtable `+0x1D4` | vtable `0x007E3EBC+0x1D4 -> 0x0070C5B0`; decompile returns `this+0x270` | Conditional |
| `+0x271` | `TechnoClass` | being-warped flag returned by vtable `+0x1D8` | vtable `0x007E3EBC+0x1D8 -> 0x0070C5C0`; decompile returns `this+0x271` | Conditional |
| `+0x520` | `BuildingClass` | `BuildingTypeClass*` | `BuildingClass::Update` decompile and asm `0x004403D4` | Yes |
| `+0x9C/+0xA0/+0xA4` | `ObjectClass`/building | raw location used by center fallback | asm `0x0044051D..0x00440545` | Conditional on warp branch |
| `+0x1580` | `BuildingTypeClass` | count used as loop upper bound; same field parsed as `MaxNumberOccupants` | asm `0x004403E0`, `0x004404E3`; prior parser docs/INI | Conditional on warp branch |
| `+0x1588 + i*8` | `BuildingTypeClass` | per-slot isometric pixel coordinate source | asm `0x00440406`, `0x00440436`; decompile | Conditional on warp branch |
| `Rules+0x344` | `RulesClass` | `[General] ChronoSparkle1` `AnimTypeClass*` | `RULESCLASS_FIELDS.csv:34`; asm reads `0x8871E0+0x344` | Conditional on warp branch |
| `WeaponType+0x110` | `WeaponTypeClass` | actual shot-triggered occupied-building anim | `Fire_At` decompile; asm `0x006FF394..0x006FF41D` | Yes for occupied shots with `OccupantAnim` |

## 3. Core Logic

### 3.1 Branch gate

`BuildingClass::Update` calls vtable `+0x1D4`; if false, it calls vtable `+0x1D8`. Only if either is true does it enter the branch at `0x004403D4`.

Verified function targets:

- Building vtable base `0x007E3EBC`; `+0x1D4` reads `0x0070C5B0`, decompiled as `TechnoClass::IsWarpingOut`, returning byte `this+0x270`.
- Building vtable base `0x007E3EBC`; `+0x1D8` reads `0x0070C5C0`, decompiled as `TechnoClass::IsBeingWarped`, returning byte `this+0x271`.
- Assembly in `BuildingClass::Update`: `0x0043FD08..0x0043FD26` calls those slots and jumps to `0x004403D4` only on nonzero.

Active in YR: Conditional. This is live for chrono/temporal building states, not for ordinary garrison occupancy.

### 3.2 Port-offset path

If `BuildingTypeClass+0x1580 != 0` and `RulesClass+0x344 != 0`, the branch loops:

```text
for i in 0 .. Type+0x1580 exclusive:
    if ((g_CurrentFrameCounter + i) % 24) == 0 and Rules+0x344 != 0:
        offset = IsometricPixelToWorld(Type+0x1588 + i*8)
        base = this->vtable+0xAC()
        coord = base + offset
        anim = AnimClass(Rules+0x344, coord, 0, 1, 0x600, 0, 0)
        anim+0x100 = -200
```

Assembly evidence:

- `0x004403E0..0x004403F6`: read `Type+0x1580`, read `Rules+0x344`, fallback if either is zero.
- `0x0044040B..0x0044041C`: `eax = g_CurrentFrameCounter + i`, signed divide by `0x18`, spawn only when remainder is zero.
- `0x00440436..0x00440445`: address `Type+0x1588 + i*8`, call `IsometricPixelToWorld @ 0x006D2070`.
- `0x0044045D..0x00440468`: call building vtable `+0xAC`; for BuildingClass this is `0x00459EF0`, returning `(Location_X-0x80, Location_Y-0x80, Location_Z)`.
- `0x00440488..0x004404C6`: allocate `0x1C8`, call `AnimClass::Constructor @ 0x00421EA0` with flags `0x600`.
- `0x004404CF`: write `anim+0x100 = 0xFFFFFF38` (`-200`).
- `0x004404D9..0x004404E9`: increment `i`, add `8` to the coordinate pointer base, loop while `i < Type+0x1580`.

Tiny details:

- The modulo uses signed `idiv 0x18`; with normal nonnegative frame counter and small nonnegative `i`, this is equivalent to `(frame + i) % 24`.
- The loop bound is `MaxNumberOccupants`, not current occupant count.
- There is no read of `CanBeOccupied`, `CanOccupyFire`, `Building+0x694` occupant count, or `Building+0x69C` current fire index in this branch.
- The branch re-reads `Rules+0x344` inside the loop before constructing each anim, even though the outer gate already checked it.
- If `operator_new` returns null, the decompile shows a potential write through zero at `anim+0x100`; this is native behavior, not a parity requirement to emulate as a crash.

Active in YR: Conditional on building warp flags plus `ChronoSparkle1` pointer and `MaxNumberOccupants > 0`.

### 3.3 Center fallback path

If `Type+0x1580 == 0` or `Rules+0x344 == 0`, the branch falls back to a center-location cadence, but still requires `Rules+0x344 != 0` before creating an anim:

```text
if g_CurrentFrameCounter % 24 == 0 and Rules+0x344 != 0:
    AnimClass(Rules+0x344, this->Location, 0, 1, 0x600, 0, 0)
```

Assembly evidence:

- `0x004404F1..0x00440500`: `g_CurrentFrameCounter % 24 == 0`.
- `0x00440502..0x0044050A`: re-check `Rules+0x344 != 0`.
- `0x0044051D..0x00440558`: copy `this+0x9C/+0xA0/+0xA4`, pass flags `0x600`, call `AnimClass::Constructor`.

Tiny detail: the center fallback does not write `anim+0x100 = -200`; the port-offset path does.

Active in YR: Conditional on building warp flags and no port-count path, with `[General] ChronoSparkle1` non-null.

### 3.4 Contrast: actual shot-triggered garrison flash

`TechnoClass::Fire_At` has the actual occupied-building shot visual:

- It selects normal weapon `Anim=` into a local anim pointer.
- It then calls vtable `+0x400` (`BuildingClass::IsOccupied`) and, if true, overwrites the anim pointer with `WeaponType+0x110`.
- It constructs `AnimClass` at the firing coordinate and for buildings with `GetOccupantCount() > 0` writes `anim+0x100 = -200`.

Evidence: `Fire_At` decompile; assembly `0x006FF320..0x006FF349` selects/falls back normal anim; `0x006FF394..0x006FF3C6` constructs the anim; `0x006FF411..0x006FF41D` calls occupant count and writes `-200` when count is positive. Earlier settled report verifies `+0x110` as `OccupantAnim`.

Active in YR: Yes for ordinary garrison shots.

### 3.5 Actual shot flash timing and lifetime

The shot-triggered `OccupantAnim` is not timed by `BuildingClass::Update`. It is a normal `AnimClass` instance constructed by `Fire_At` with fixed arguments:

```text
AnimClass::Constructor(animType, &fireCoord, delay=0, loopCount=1, drawFlags=0x600, zAdjust=0, reverse=0)
```

`delay=0` starts the anim immediately through `AnimClass::Middle`. The frame cadence and deletion are then the generic `AnimTypeClass` / `AnimClass::AI` contract:

- `AnimTypeClass::ReadINI @ 0x00427D00` reads `Rate=` and stores internal frame delay as `900 / Rate` when `Rate > 0`; `Rate <= 0` stores `0`.
- `AnimTypeClass::Constructor @ 0x00427530` initializes `Rate` to `1`, `Start/LoopStart/LoopEnd/End` to `0`, and `LoopCount` to `0`.
- `AnimClass::Constructor @ 0x00421EA0` copies the type rate into the anim's frame-delay fields, computes `LoopCountRemaining = type->LoopCount * loopCount`, clamps that remaining-loop byte to at least `1`, and calls `Middle` immediately when delay is zero.
- `AnimClass::AI @ 0x00423AC0` advances frames when the countdown expires, honors the type's `End`, `LoopStart`, `LoopEnd`, `LoopCount`, `Next`, `Shadow`, `PingPong`, `RandomRate`, and related generic fields, then marks/deletes the anim when its lifecycle completes.

Stock YR UC shot anim sections in `artmd.ini` (`[UCFLASH]`, `[UCCONS]`, `[UCINIT]`) contain only `Layer=ground` and `Translucent=yes` in the scoped scan (`artmd.ini:16131..16141`). They do not define `Rate=`, `LoopStart=`, `LoopEnd=`, `LoopCount=`, `Start=`, `End=`, or `Next=`, so stock cadence uses the constructor/default `Rate=1` internal logic tick and generic SHP/default end behavior. For modded `OccupantAnim` sections, the art metadata must drive cadence/lifetime.

Active in YR: Yes for each successful ordinary occupied-building shot with non-null `WeaponType+0x110`.

## 4. INI Keys

| INI key | Stock value | Binary field | Effect in this slice | Active in YR |
|---|---:|---|---|---|
| `[General] ChronoSparkle1` | `CHRONOSK` in `rulesmd.ini:554`; `rules.ini:546` | `RulesClass+0x344` | Anim used by this `BuildingClass::Update` branch | Conditional on warp flags |
| `CanBeOccupied` | many stock buildings set `yes` | `BuildingType+0x157B` | Not read by the `Update` branch | Yes elsewhere |
| `CanOccupyFire` | many stock garrisons set `yes` | `BuildingType+0x157C` | Not read by the `Update` branch; read by `IsOccupied` | Yes elsewhere |
| `MaxNumberOccupants` | stock values vary | `BuildingType+0x1580` | Loop upper bound in the warp sparkle branch | Conditional here; yes for garrison data |
| `MuzzleFlash0..N` | art offsets | `BuildingType+0x1588 + i*8` | Offset source in the warp sparkle branch | Conditional here; yes as art data |
| `OccupantAnim` | e.g. `UCFLASH`, `UCCONS`, `UCINIT` | `WeaponType+0x110` | Actual shot-triggered occupied-building anim in `Fire_At` | Yes for weapons that define it |

## 5. Integration Points

`BuildingClass::Update` is a live per-building update function. The scoped branch runs after damage-fire timer/audio/light update and docked-object update, but before normal AI, health-zero cleanup, delayed fire, auto-sell, repair/power, and target validation. When the warp branch is taken it returns early, except for a `+0x2B4` pointer case that jumps to the final `vtable+0x3C8(0)` gate.

The branch does not consult map visibility, shroud, owner, damage state, dead state, current occupant count, or `CanOccupyFire`. Its only direct gates are the two warp flags plus the anim/count checks described above.

## 6. Current Rust Implementation Status

Current Rust has shot-triggered occupied-building `OccupantAnim` surfaces:

- `src/sim/world/mod.rs` defines `SimFireEvent.garrison_muzzle_index` and `occupant_anim`.
- `src/sim/combat/mod.rs` fills those fields from garrison fire events.
- `src/app_building_anim.rs::tick_garrison_muzzle_flashes` spawns one-shot flashes from pending fire events using `occupant_anim`.
- `src/app_fire_effects.rs` resolves garrison fire origins from art muzzle ports.
- `src/app_instances/overlays.rs` renders active garrison muzzle flash instances.

No Rust surface found in the scan implements a continuous, non-shot garrison ambient flash. That is correct for ordinary occupied-garrison fire. If Rust lacks chrono/temporal building sparkle rendering at garrison port offsets, that is a chrono visual gap, not a garrison combat/muzzle cadence gap.

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| `BuildingClass::Update` warp visual branch | verified | decompile `0x0043FB20`; asm `0x004403D4..0x0044055D` | none for scoped branch |
| Branch entry gates `vtable+0x1D4/+0x1D8` | verified | vtable reads `0x007E4090/0x007E4094`; decompile `0x0070C5B0`, `0x0070C5C0` | full chrono lifecycle out of scope |
| Cadence formula | verified | asm `0x0044040B..0x0044041C`, `0x004404F1..0x00440500` | none |
| Port loop bounds/stride | verified | asm `0x00440406`, `0x004404D9..0x004404E9` | none |
| Coordinate source | verified | asm `0x00440436..0x00440468`; `BuildingClass::GetRenderCoords @ 0x00459EF0` | exact screen draw ordering of resulting AnimClass is out of scope |
| Anim source `Rules+0x344` | verified | asm `0x004403EE`, `0x00440422..0x00440430`, `0x00440549..0x00440555`; `RULESCLASS_FIELDS.csv:34` | none |
| Ordinary shot-triggered `OccupantAnim` contrast | verified-enough-for-separation | `Fire_At` decompile; asm `0x006FF320..0x006FF41D`; prior settled report | weapon selection itself out of scope |
| Rust shot-triggered surface | verified by source scan | `src/sim/world/mod.rs`, `src/sim/combat/mod.rs`, `src/app_building_anim.rs`, `src/app_fire_effects.rs`, `src/app_instances/overlays.rs` | chrono sparkle Rust surface not exhaustively scanned |

## 8. Open Questions - Final State of the Investigation Log

- `[RESOLVED] OQ-01 - Is the `BuildingClass::Update` branch entered by garrison occupancy? -> No; it is gated by `TechnoClass+0x270/+0x271` warp flags via vtable `+0x1D4/+0x1D8`.` (evidence: `0x0043FD08..0x0043FD26`, `0x0070C5B0`, `0x0070C5C0`)
- `[RESOLVED] OQ-02 - Is cadence every 24 frames? -> Yes when the branch is active: center fallback uses `frame % 24`, port path uses `(frame + port) % 24`.` (evidence: `0x0044040B..0x0044041C`, `0x004404F1..0x00440500`)
- `[RESOLVED] OQ-03 - What is the port iteration bound? -> `0 <= port < BuildingType+0x1580`; it is not current occupant count.` (evidence: `0x004403E0`, `0x004404E3..0x004404E9`)
- `[RESOLVED] OQ-04 - What coordinate source is used? -> Port path uses `IsometricPixelToWorld(Type+0x1588+port*8)` plus vtable `+0xAC` render coords; center fallback uses raw `this+0x9C/+0xA0/+0xA4`.` (evidence: `0x00440436..0x00440468`, `0x0044051D..0x00440545`, `0x00459EF0`)
- `[RESOLVED] OQ-05 - What anim source is used? -> `RulesClass+0x344`, `[General] ChronoSparkle1=CHRONOSK` in stock YR, not weapon `OccupantAnim`.` (evidence: `0x004403EE`, `0x004404BD..0x004404C6`, `RULESCLASS_FIELDS.csv:34`, `rulesmd.ini:554`)
- `[RESOLVED] OQ-06 - Does this branch read `CanOccupyFire`? -> No. `CanOccupyFire` is part of `BuildingClass::IsOccupied`, not the `Update` branch.` (evidence: `0x004403D4..0x0044055D`, `0x00458DD0..0x00458DFE`)
- `[RESOLVED] OQ-07 - Does this branch require occupant count > 0? -> No occupant-count read occurs in the `Update` branch.` (evidence: `0x004403D4..0x0044055D`; contrast `GetOccupantCount @ 0x004581F0`)
- `[RESOLVED] OQ-08 - Does this branch use max occupants? -> Yes, but only as count/offset list for chrono sparkle placement.` (evidence: `0x004403E0`, `0x004404E3`)
- `[RESOLVED] OQ-09 - Is there a visibility/shroud gate? -> No visibility or shroud predicate appears in the scoped `Update` branch.` (evidence: decompile `0x0043FB20`; asm `0x004403D4..0x0044055D`)
- `[RESOLVED] OQ-10 - Is there a damaged/dead gate? -> No local health/dead gate in the branch; the later `Health==0` cleanup is skipped by this early-return branch.` (evidence: decompile ordering `0x0043FB20`; branch return at `0x0044055D..0x0044057A`)
- `[RESOLVED] OQ-11 - Is actual `OccupantAnim` still shot-triggered? -> Yes; `Fire_At` uses `WeaponType+0x110` for occupied buildings after successful fire setup.` (evidence: `0x006FF320..0x006FF41D`)
- `[RESOLVED] OQ-12 - Is this TS legacy only? -> No; chrono/temporal building warp visuals are live YR-capable behavior, but conditional on warp flags. It is not standard idle occupied-garrison behavior.` (evidence: `rulesmd.ini:548..554`, `RULESCLASS_FIELDS.csv:28..34`, branch gates above)
- `[DEFERRED] OQ-13 - Which standard scenario produces a warping occupied civilian building?` (category: out-of-scope; reason: this slot only needed to disprove ordinary continuous garrison muzzle cadence; next-step-if-pursued: trace Chronosphere/TemporalClass building target eligibility and runtime flags)

Adversarial checks answered from evidence:

- Empty garrison? No spawn unless warp flags are set; occupant count is not read.
- Full garrison? Port path loops to max slots, not actual filled slots.
- `CanOccupyFire=no` with occupants? This branch does not care; actual shot path does through `IsOccupied`.
- Hidden/unseen building? No scoped visibility gate found.
- Dead building? No local health gate before early return; later death handling is bypassed while the warp branch returns.

## 9. Visual/UI Composition Ledger

| Order | Function / address | Condition / flag proof | Asset / frame | Rect / anchor | Palette / convert | Active for target? | Role |
|---|---|---|---|---|---|---|---|
| 1 | `BuildingClass::Update` `0x0043FD08..0x004403D4` | `IsWarpingOut || IsBeingWarped` | none yet | none | none | Conditional | branch gate |
| 2 | `IsometricPixelToWorld @ 0x006D2070` | only port path with `Type+0x1580 != 0 && Rules+0x344 != 0` | `MuzzleFlashN` coordinate data, not SHP | `Type+0x1588+i*8` | world conversion | Conditional | anchor conversion |
| 3 | `BuildingClass::GetRenderCoords @ 0x00459EF0` | port path | none | `(Location_X-128, Location_Y-128, Location_Z)` | none | Conditional | anchor base |
| 4 | `AnimClass::Constructor @ 0x00421EA0` | modulo hit and `Rules+0x344 != 0` | `[General] ChronoSparkle1` anim type | port coord or raw building location | normal AnimClass pipeline | Conditional | chrono sparkle overlay |
| 5 | write `anim+0x100=-200` | port path only | same anim | depth adjustment | normal AnimClass pipeline | Conditional | z/depth adjustment |

Asset role matrix:

| Asset | Loaded | Drawn | Visible in target | Content/preview | Chrome/container | Overlay | Transition-only | Inactive | Evidence |
|---|---|---|---|---|---|---|---|---|---|
| `CHRONOSK` (`ChronoSparkle1`) | yes in stock YR | conditional | yes when building warp flags active | no | no | yes | chrono/temporal | no | `rulesmd.ini:554`, `Rules+0x344`, `0x004404BD..0x004404C6` |
| `UCFLASH` / weapon `OccupantAnim` | yes for weapons that define it | shot-triggered only | yes on actual occupied shots | no | no | yes | no | inactive for `Update` branch | `Fire_At @ 0x006FF320..0x006FF41D`, weapon data |

## 10. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| Ordinary occupied buildings do not get a separate `BuildingClass::Update` ambient/continuous garrison muzzle flash. The suspected branch is chrono warp gated. | `0x0043FD08..0x0043FD26`, `0x0070C5B0`, `0x0070C5C0`, `0x004403D4..0x0044055D` | none for current shot-triggered garrison flash; Rust currently appears to have only fire-event-driven `OccupantAnim` | `src/app_building_anim.rs::tick_garrison_muzzle_flashes`, `src/sim/combat/mod.rs`, `src/sim/world/mod.rs` | Preserve shot-triggered garrison `OccupantAnim`; do not add a 24-frame ambient garrison flash for normal occupied buildings | Occupied CABHUT with one GI idling between ROF shots shows no extra continuous `CHRONOSK`/UCFLASH cadence; flashes occur only on actual fire events | `garrison_no_ambient_update_muzzle_flash_between_shots` -> risk: adding ambient flashes would make garrisons visibly fire when no shot occurred |
| If a building is warping/being warped, the branch can spawn `[General] ChronoSparkle1` every 24 frames at either raw building location or all `MaxNumberOccupants` port offsets with `(frame+i)%24==0`. | asm `0x004403D4..0x0044055D`; `RULESCLASS_FIELDS.csv:34`; `rulesmd.ini:554` | unchecked / likely missing as a chrono visual surface | future chrono visual layer, not garrison combat layer; possibly app animation overlay path | Implement as chrono sparkle visual gated by Techno warp flags and `[General] ChronoSparkle1`, using `MaxNumberOccupants`/`MuzzleFlashN` only as coordinate data when present | Warping building with `MaxNumberOccupants=3` spawns sparkles at ports `i=0,1,2` on frames satisfying `(frame+i)%24==0`, not based on occupant count | `chrono_warp_building_sparkles_use_port_offsets_and_global_frame_stagger` -> risk: misclassifying this as garrison fire ties visual cadence to combat and wrong anim |
| Actual occupied shot flash remains `WeaponType+0x110` (`OccupantAnim`) and not `Rules+0x344`. | `Fire_At` decompile and asm `0x006FF320..0x006FF41D`; prior garrison fire-index report | implemented in current Rust event/render surface | `src/sim/combat/mod.rs`, `src/app_building_anim.rs`, `src/app_fire_effects.rs` | Keep the shot-triggered path separate from chrono sparkle path | GI in occupied building fires: visible shot flash uses weapon `OccupantAnim=UCFLASH`; changing `[General] ChronoSparkle1` should not alter shot flash | `garrison_fire_uses_weapon_occupant_anim_not_chronosparkle1` -> risk: using global chrono anim for shots breaks weapon-specific presentation |
| Shot-triggered `OccupantAnim` timing/lifetime is generic `AnimClass`: `Rate=` converts to `900/Rate`, absent stock UC `Rate=` leaves default internal `1`, and `End/Loop/Next/Shadow` govern deletion. | `AnimTypeClass::ReadINI @ 0x00427D00`; `AnimTypeClass::Constructor @ 0x00427530`; `AnimClass::Constructor @ 0x00421EA0`; `AnimClass::AI @ 0x00423AC0`; `artmd.ini:16131..16141` | current Rust `tick_garrison_muzzle_flashes` uses hardcoded `67ms` and raw SHP frame count, which matches stock-default cadence only approximately and misses modded/generic `AnimType` lifecycle | `src/app_building_anim.rs`, `src/rules/art_data.rs`, future generic `AnimClass` model | Parse/use selected `AnimType` timing and loop metadata for garrison `OccupantAnim` flashes instead of hardcoding cadence/lifetime | Modded `OccupantAnim=MYUC` with `Rate=300`, `LoopStart=0`, `LoopEnd=3`, `LoopCount=1` advances every 3 logic ticks and removes according to native loop/end rules | `garrison_occupant_anim_uses_art_rate_and_loop_metadata` -> risk: raw frame-count expiry drifts for `End`, `LoopCount`, `Next`, `Shadow`, and modded anims |

### Stale Docs / Follow-up Docs

- `C:/Users/enok/Documents/ra2-rust-game/docs/research/GARRISON_SYSTEM_GHIDRA_REPORT.md`: replace the section beginning `### Muzzle Flash -- spawned in BuildingClass::Update` through the end of `14f. Muzzle Flash Spawning in BuildingClass::Update` with: "The `BuildingClass::Update @ 0x0043FB20` branch at `0x004403D4..0x0044055D` is not a continuous occupied-garrison muzzle-flash path. It is gated by `TechnoClass::IsWarpingOut` / `IsBeingWarped` (`+0x270/+0x271`) and spawns `[General] ChronoSparkle1` from `RulesClass+0x344`. When `BuildingType+0x1580 > 0`, it uses `MuzzleFlashN` offsets as chrono sparkle anchor points with `(g_CurrentFrameCounter + port) % 24 == 0`; otherwise it may spawn at the building location every 24 frames. Actual occupied-garrison shot flashes are produced by `TechnoClass::Fire_At` using `WeaponType+0x110` (`OccupantAnim`)."
- `C:/Users/enok/Documents/ra2-rust-game/docs/research/GARRISON_IMPLEMENTATION_PLAN.md`: replace "The original spawns muzzle flash AnimClass at each fire port every 24 frames" and the following "NOT driven by individual Fire_At events" claim with: "Do not implement a normal occupied-garrison ambient muzzle flash from `BuildingClass::Update`. Fresh Ghidra verification shows that 24-frame branch is chrono/temporal sparkle rendering using `[General] ChronoSparkle1`, not garrison combat. For garrison combat, keep `Fire_At`/`SimFireEvent` shot-triggered `WeaponType+0x110` (`OccupantAnim`) flashes."
- `C:/Users/enok/Documents/ra2-rust-game/docs/research/BUILDINGCLASS_UPDATE_AI_TICK_GHIDRA_REPORT.md`: minor wording replacement only: replace "`RulesClass+0x344` (AnimTypeClass*) -- `WarpOut` anim type" with "`RulesClass+0x344` (AnimTypeClass*) -- `[General] ChronoSparkle1` (`CHRONOSK` in stock YR); not `WarpOut`, which is `RulesClass+0x33C`."

## Negative Facts / Do Not Do

- Do not add a continuous occupied-garrison muzzle flash that runs every 24 frames during normal idle/guard state. Evidence: `BuildingClass::Update` branch gate is `+0x270/+0x271`, not occupancy (`0x0043FD08..0x004403D4`).
- Do not use `[General] ChronoSparkle1` / `Rules+0x344` as a garrison shot muzzle animation. Evidence: `Rules+0x344` branch is chrono-gated; shot path uses `WeaponType+0x110` in `Fire_At`.
- Do not bound the `Update` branch by current occupant count. Evidence: loop compares against `Type+0x1580` (`0x004404E3..0x004404E9`) and never reads `Building+0x694`.
- Do not gate the `Update` branch on `CanOccupyFire`. Evidence: `CanOccupyFire` is read in `BuildingClass::IsOccupied @ 0x00458DD0`, but that function is not called by the `Update` branch.
- Do not describe the coordinate base as building center without qualification. Port path uses `BuildingClass::GetRenderCoords` (`Location_X-0x80`, `Location_Y-0x80`, `Location_Z`) plus converted offset; only fallback uses raw location.
- Do not hardcode the general garrison shot flash lifetime as raw SHP frame count at 67ms. Evidence: native shot flash is a normal `AnimClass`; generic `AnimType` `Rate/End/Loop/Next/Shadow` metadata drives cadence and deletion.

## Remaining Uncertainty

- None for the scoped question. The only deferred item is out of scope: exact stock scenario coverage for a warping occupied/garrisonable building.

## Sources

- Ghidra decompile: `BuildingClass::Update @ 0x0043FB20`
- Local disassembly from retail `gamemd.exe`: `0x004403D4..0x0044055D`, `0x0043FD08..0x0043FD26`, `0x006FF320..0x006FF41D`, `0x00458DD0..0x00458DFE`
- Ghidra decompile: `TechnoClass::IsWarpingOut @ 0x0070C5B0`, `TechnoClass::IsBeingWarped @ 0x0070C5C0`, `BuildingClass::GetRenderCoords @ 0x00459EF0`, `BuildingClass::IsOccupied @ 0x00458DD0`, `BuildingClass::GetOccupantCount @ 0x004581F0`
- `C:/Users/enok/Documents/ra2-rust-game/docs/research/RULESCLASS_FIELDS.csv`
- `C:/Users/enok/Documents/ra2-rust-game/ini/rulesmd.ini`
- `C:/Users/enok/Documents/ra2-rust-game/ini/rules.ini`
- Current Rust scan: `src/sim/world/mod.rs`, `src/sim/combat/mod.rs`, `src/app_building_anim.rs`, `src/app_fire_effects.rs`, `src/app_instances/overlays.rs`
