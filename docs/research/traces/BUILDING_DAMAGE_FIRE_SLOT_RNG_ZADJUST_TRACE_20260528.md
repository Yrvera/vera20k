# Building Damage-Fire Slot/RNG/ZAdjust Trace - 2026-05-28

## Scope

Scenario: stock `GACNST` crosses the ordinary building damage-fire threshold during an update.

This trace is intentionally narrow: persistent building damage-fire slots, initial fire type RNG,
per-slot start-frame RNG, native slot indices, computed `ZAdjust`, render depth consumption, and
removal when repaired above threshold. Adjacent destruction debris, Sparky damage particles,
garrison visuals, terrain fire, and superweapon `CreateFireAnim` are not traced here.

## Concrete Inputs

Retail INI inputs used for the concrete scenario:

- `ini/rulesmd.ini:519`: `DamageFireTypes=FIRE01,FIRE02,FIRE03`, count = 3.
- `ini/rulesmd.ini:752`: `ConditionRed=25%`.
- `ini/rulesmd.ini:753`: `ConditionYellow=50%`.
- `ini/rulesmd.ini:11622-11627`: `GACNST`, `Strength=1000`.
- `ini/artmd.ini:1599-1620`: `GACNST`, `Foundation=4x4`, `DamageFireOffset0=-24,-1`, `DamageFireOffset1=64,36`.
- `ini/artmd.ini:16018-16035`: `FIRE01/02/03`, `Rate=450`, `LoopCount=-1`.

Computed scenario thresholds:

- Ordinary yellow threshold: `1000 * 50% = 500 HP`; crossing from `501 -> 500` enters the fire state.
- Repair removal threshold for the same ordinary case: `501 HP` because `501 / 1000 > 0.5`.
- `GACNST` valid damage-fire offsets: 2 contiguous slots, indices `0` and `1`.
- `GACNST` foundation sum: `4 + 4 = 8`.
- Slot 0 `zAdjust`: `(((-1 + 8 * -15) * 3) >> 1) - 10 = -192`, clamp unchanged.
- Slot 1 `zAdjust`: `(((36 + 8 * -15) * 3) >> 1) - 10 = -136`, clamp unchanged.

## Evidence

gamemd evidence:

- `BuildingClass::Update @ 0x0043FB20` is active in standard YR and reads `Type+0x157B`.
  `0` selects `Rules+0x1700` (`ConditionYellow`), nonzero selects `Rules+0x1708`
  (`ConditionRed`). It calls `BuildingClass::CreateDamageFireAnims @ 0x0043C0D0` on
  cached fire-state false->true, and uninitializes all 8 slots on true->false.
  Evidence: read-only Ghidra decompile in this trace; also
  `docs/research/ANIMCLASS_BUILDING_OBJECT_DAMAGE_RUNTIME_SPAWNS_GHIDRA_REPORT.md:39`.
- `BuildingClass::CreateDamageFireAnims @ 0x0043C0D0` is active in standard YR. It reads
  `Rules+0x2B0`, consumes `RandomRanged(0,count-1)` for the initial type index, scans
  slots `0..7`, stops at first sentinel or occupied slot, constructs real `AnimClass`
  objects with flags `0x600`, stores them at `BuildingClass+0x5C8+slot*4`, writes the
  computed `ZAdjust`, then consumes `RandomRanged(0,frame_count-1)` for each positive
  frame count. Evidence: read-only Ghidra decompile in this trace; also
  `docs/research/ANIMCLASS_BUILDING_OBJECT_DAMAGE_RUNTIME_SPAWNS_GHIDRA_REPORT.md:41-43`.
- `Random::RandomRanged @ 0x0065C7E0` is active in standard YR, inclusive on both ends,
  equal bounds do not draw, and ordinary ranges use mask/rejection over the 250-word
  XOR-lag state. Evidence:
  `docs/research/RANDOM_RANDOMRANGED_0065C7E0_GHIDRA_REPORT.md:30-39`.
- `AnimClass::DrawIt @ 0x00422CA0` standard branch passes an integer depth expression
  using instance `ZAdjust`: `YDrawOffset + AnimClass.ZAdjust - Tactical::AdjustForZ() - 2`.
  Evidence: `docs/research/OCCUPANTANIM_ANIMCLASS_LIFECYCLE_DRAWIT_DEPTH_GHIDRA_REPORT.md:148-161`.

Rust evidence:

- `src/app_building_anim.rs:79-198` runs `tick_damage_fire_overlays` after the fixed sim
  advance, finds structures under the current threshold, builds spawn plans, then inserts
  `DamageFireOverlays` in the same app update.
- `src/app_building_anim.rs:247-281` creates slot entries in one helper call, consumes one
  RNG draw for the first type index, then one draw per slot frame, stores slot index,
  pixel offset, frame, total frame count, rate, and `z_adjust`.
- `src/app_building_anim.rs:284-289` computes the same `z_adjust` formula for the two
  concrete `GACNST` slots: `-192` and `-136`.
- `src/app_building_anim.rs:291-299` always returns `ConditionYellow` because the raw
  `BuildingType+0x157B` selector is not exposed.
- `src/app_instances/overlays.rs:106-160` renders the overlay entries and feeds
  `fire.z_adjust` into sprite depth.
- `src/app_instances/overlays.rs:547-552` maps the native integer adjust into a normalized
  float depth bias around neutral value `1000`.
- `src/sim/components.rs:644-677` stores the current overlay state as app-side
  `DamageFireOverlays`, not native `AnimClass` objects.
- `src/app_sim_tick.rs:176-198` calls damage-fire ticking after `advance_fixed_simulation`.

## Pipeline Verdicts

| Stage | Verdict | Computed Rust output | Computed gamemd output | Notes |
|---|---:|---|---|---|
| Retail data for concrete stock building | PASS | `GACNST Strength=1000`, offsets `(-24,-1),(64,36)`, foundation `4x4`, fire type count `3` | Same retail INI rows consumed by gamemd reports | `*md` file has priority and matches base for this section. |
| Ordinary threshold crossing | PASS | `500 / 1000 = 0.5`, current Rust yellow threshold `0.5` spawns at `<=` | `ConditionYellow=0.5`; update branch sets damaged when health ratio is not greater than threshold | PASS only for the ordinary yellow-threshold path, not for the unresolved selector. |
| `BuildingType+0x157B` threshold selector | UNCHECKED | Rust passes `None` and always selects `ConditionYellow` | Binary reads `Type+0x157B`; zero -> yellow, nonzero -> red | The raw `GACNST` byte was not computed from gamemd state in this run. Conflicting docs tie the byte to garrison occupancy, but this trace did not re-audit parser/defaults. |
| Same-update slot creation | PASS | `create_damage_fire_slot_anims` returns 2 entries in one call for the 2 offsets | `CreateDamageFireAnims` scans slots in one call and stops at first sentinel after slot 1 for `GACNST` | Both create all valid contiguous slots in the triggering update call. |
| Slot index carry | PASS | Stored `slot=0`, `slot=1` | Stored pointers at `+0x5C8+0*4`, `+0x5C8+1*4` | Literal slot indices match the concrete two-slot input. |
| Initial fire type RNG and wrap | UNCHECKED | Code consumes `rng.next_range_u32(3)` then wraps indices per slot | gamemd consumes `RandomRanged(0,2)` then wraps indices per slot | Call order and bounds match; no concrete shared RNG state/output was computed for this scenario. |
| Per-slot start-frame RNG | UNCHECKED | Code consumes one draw per slot using loaded `effect_frame_counts` | gamemd consumes one draw per slot using `AnimType+0x2C0` frame count | No concrete FIRE01/02/03 SHP frame counts and shared RNG outputs were computed here. |
| `ZAdjust` computation and carry | PASS | Slot 0 `-192`, slot 1 `-136`, stored in each `DamageFireAnim` | Same formula at `0x0043C0D0` produces `-192`, `-136`; clamp unchanged | Literal integer equality for the two concrete slots. |
| Render depth uses `ZAdjust` | FAIL | `depth = base_depth + (1000 - z_adjust) * 0.000001`, clamped float | `AnimClass::DrawIt`: integer `YDrawOffset + ZAdjust - Tactical::AdjustForZ() - 2` | Rust does apply the value to depth, but not with the verified native integer expression. Exact sort equality is unproven and mechanism differs. |
| Native `AnimClass` object/lifecycle | NOT-IMPLEMENTED | App-side `DamageFireOverlays` vector on the building entity | Real `AnimClass` objects allocated, globally registered, AI/lifecycle/rendered through `AnimClass` | This affects constructor ordering, object-array iteration, sound/lifecycle, and exact draw path. |
| Repair removal above threshold | PASS | At `501 / 1000 > 0.5`, `damage_fire_overlays = None` in the same app update | On true->false cached byte transition, gamemd uninitializes non-null slots and clears pointers | PASS only for the ordinary yellow-threshold path; selector-specific red-threshold cases remain covered by the selector UNCHECKED verdict. |

## Failures And Gaps

1. Render depth is not native exact.
   Current Rust uses a normalized float bias around neutral `1000`, while the verified
   `AnimClass::DrawIt` path passes an integer expression using `YDrawOffset`, instance
   `ZAdjust`, `Tactical::AdjustForZ()`, and a `-2` bias. The player-visible risk is
   incorrect ordering of damage fires against building bodies, walls, and nearby shapes.

2. Persistent fires are not real native `AnimClass` objects.
   Current Rust stores app-layer overlays on the building entity. Native gamemd allocates
   and registers real `AnimClass` objects, stores their pointers in the building's 8 native
   slots, and then relies on generic `AnimClass` AI/draw/lifetime. The player-visible risk
   is different animation cadence, sound/lifecycle side effects, draw ordering, and global
   object iteration behavior.

3. Exact RNG outputs remain unchecked.
   Rust now calls a gamemd-shaped `SimRng` helper for the same bounds/order used by the
   damage-fire path, but this trace did not compute a shared pre-spawn RNG state and did
   not compare literal first-type and per-slot start-frame values against gamemd.

4. `BuildingType+0x157B` remains unchecked for this concrete stock building.
   The active binary selector is verified, but this run did not compute the raw `GACNST`
   byte from gamemd state. Rust explicitly falls back to yellow until that byte is exposed.

## Adjacent Findings

- `src/sim/game_entity.rs:546-556` and several sim callers maintain
  `building_damage_state_active` using integer `ConditionYellow` only. This is adjacent to
  the same selector issue but not traced as a separate gameplay system here.
- `src/app_building_anim.rs:217-225` can advance freshly spawned overlay frames during the
  same app tick if `dt_ms` is already at or above the rate. For ordinary 66 ms fixed ticks
  and `FIRE01/02/03 Rate=450`, the concrete first tick does not advance, but hitch/batched
  behavior was not traced here.

## Verdict Tally

PASS: 6 | FAIL: 1 | UNCHECKED: 3 | NOT-IMPLEMENTED: 1

## Status

COMPLETE
