# CAWASH19 ActiveAnimGarrisoned Stock-Inactive Postfix Trace

Date: 2026-05-27

Scenario: stock Yuri's Revenge `CAWASH19` has `ActiveAnimGarrisoned=CAWA19_AG` in art data, but the standard rules entry comments out the `CanBeOccupied` garrison flags. Verify current Rust after the postfix keeps `CAWA19_AG` as a replacement variant on the base `ActiveAnim` slot and does not emit it as an extra stock overlay.

Scope: CAWASH19 only. This trace does not generalize to other garrisonable buildings or damaged active anims.

## Pipeline

`rulesmd.ini` stock flags -> `artmd.ini` active anim fields -> Rust rules/art parsing -> spawn passenger role -> render `is_garrisoned` gate -> selected building anim key -> screen.

## Evidence

- Stock YR rules data: `ini/rulesmd.ini:14589..14614` defines `[CAWASH19]` with `TechLevel=-1`, `Strength=500`, and comments out `CanBeOccupied=yes`, `MaxNumberOccupants=10`, `DistributedFire=yes`, and `CanOccupyFire=yes`.
- Stock YR art data: `ini/artmd.ini:8970..8978` defines `ActiveAnim=CAWA19_A`, `ActiveAnimDamaged=CAWA19_AD`, and `ActiveAnimGarrisoned=CAWA19_AG`; `ini/artmd.ini:9012..9021` defines `CAWA19_AG` as `Image=CAWA19_A`, `Start=33`, `LoopStart=33`, `LoopEnd=48`, `LoopCount=-1`, `Rate=220`.
- Verified gamemd research: `docs/research/GARRISON_OCCUPIED_BUILDING_VISUAL_STATE_GHIDRA_REPORT.md:88..115` says `FUN_00458330 @ 0x00458330` checks already-live anim-slot pointers and selects empty/occupied/damaged strings as slot replacements, not extra overlays.
- Verified active-YR stock data finding: `docs/research/GARRISON_OCCUPIED_BUILDING_VISUAL_STATE_GHIDRA_REPORT.md:116..128` says the only stock `ActiveAnimGarrisoned=` key found is CAWASH19, but CAWASH19 is not active as a standard YR `CanBeOccupied` garrison because the rules flags are commented out.
- Rust object parser: `src/rules/object_type.rs:1034` defaults missing `CanBeOccupied` to `false`; `src/rules/object_type.rs:1054` defaults missing `MaxNumberOccupants` to `0`.
- Rust art parser after postfix: `src/rules/art_data.rs:965..971` does not list `ActiveAnimGarrisoned` as an independent building anim key; `src/rules/art_data.rs:1007..1016` stores `ActiveAnimGarrisoned` as `garrisoned_variant` on the parsed `ActiveAnim` slot.
- Rust render after postfix: `src/app_instances/shp.rs:518..546` selects the garrisoned variant only when `is_garrisoned` is true; otherwise it returns the base `anim_type`.
- Rust spawn/render gate: `src/sim/world/world_spawn.rs:407..414` creates garrison cargo only when `can_be_occupied && max_number_occupants > 0`; with stock CAWASH19 this is `false && 0`, so no cargo role is created. `src/app_instances/shp.rs:290` computes `is_garrisoned` from non-empty cargo.
- Rust atlas after postfix: `src/render/sprite_atlas.rs:633..672` loads `garrisoned_variant` frames as variant assets, but asset availability does not make the variant an extra rendered overlay.

## Stage Results

| Stage | gamemd output | Rust output | Verdict |
|---|---:|---:|---|
| Stock rules garrison flags | `CanBeOccupied=0`, `MaxNumberOccupants=0` because keys are commented/defaulted | `can_be_occupied=false`, `max_number_occupants=0` by missing-key defaults | PASS |
| Stock art field parse | Base active slot string `CAWA19_A`; garrisoned replacement string `CAWA19_AG`; independent extra occupied overlay count `0` | one `BuildingAnimConfig` for `CAWA19_A`; `garrisoned_variant=Some(CAWA19_AG)`; independent extra overlay count `0` | PASS |
| Standard spawn/passenger eligibility | stock CAWASH19 is not a `CanBeOccupied` garrison target; no standard occupant cargo slot from the stock garrison path | `false && 0` prevents `PassengerRole::Transport`; cargo role count `0` | PASS |
| Render variant selection in stock state | no occupied active-slot replacement is reached; selected stock active key remains `CAWA19_A`, not `CAWA19_AG` | `is_garrisoned=false`, `building_damage_state_active=false`; `selected_building_anim_view` returns `CAWA19_A` | PASS |
| Final pixel capture | not captured in this read-only trace | not captured in this read-only trace | UNCHECKED |

## Verdict

Current Rust preserves the scoped post-fix behavior for stock CAWASH19: `CAWA19_AG` is retained as a replacement variant on the base `ActiveAnim` slot, not parsed as a second overlay, and stock standard-YR CAWASH19 does not activate the garrisoned active anim path because `CanBeOccupied=0` and `MaxNumberOccupants=0`.

No FAIL or NOT-IMPLEMENTED finding was found for this concrete scenario. Final pixel equality remains UNCHECKED because this slot did not run a native/Rust screenshot or sprite-instance capture, and the hard constraint allowed only one report file write.

## Adjacent Findings

- `src/app_instances/shp.rs:518..546` selects `garrisoned_variant` from `is_garrisoned` alone and does not re-check `can_be_occupied`; stock CAWASH19 does not reach this because no cargo role is created. Impossible hand-built entity state is outside this trace.
- Damaged replacement priority over garrisoned replacement is outside this trace.

## Return Tally

PASS: 4 | FAIL: 0 | UNCHECKED: 1 | NOT-IMPLEMENTED: 0
