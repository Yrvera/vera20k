# CAWASH19 ActiveAnimGarrisoned Stock-Inactive Trace

Date: 2026-05-27

Scenario: In stock Yuri's Revenge INI data, `[CAWASH19]` in `artmd.ini` has `ActiveAnimGarrisoned=CAWA19_AG`, but `[CAWASH19]` in `rulesmd.ini` has `CanBeOccupied`, `MaxNumberOccupants`, `DistributedFire`, and `CanOccupyFire` commented out. Trace whether standard YR treats CAWASH19 as an active `CanBeOccupied` garrison visual case, and compare to current Rust parser/render behavior.

Scope: CAWASH19 only. This does not trace active garrison visuals on `CanBeOccupied=yes` buildings such as CAGAS01, CAMOV02, or CASEAT02.

## Pipeline

Rules INI data -> art INI data -> rules/art parser -> entity passenger/garrison state -> building anim slot selection -> screen pixels.

## Evidence

- Stock rules data: `ini/rulesmd.ini:14589..14612` defines `[CAWASH19]` and comments out `CanBeOccupied=yes`, `MaxNumberOccupants=10`, `DistributedFire=yes`, and `CanOccupyFire=yes`.
- Stock art data: `ini/artmd.ini:8970..8978` defines `[CAWASH19]` with `ActiveAnim=CAWA19_A`, `ActiveAnimDamaged=CAWA19_AD`, and `ActiveAnimGarrisoned=CAWA19_AG`; `ini/artmd.ini:9012..9021` defines `[CAWA19_AG]` using image `CAWA19_A`, `Start=33`, `LoopStart=33`, `LoopEnd=48`.
- Verified gamemd research: `docs/research/GARRISON_OCCUPIED_BUILDING_VISUAL_STATE_GHIDRA_REPORT.md:125..127` states the only active stock `ActiveAnimGarrisoned` key is CAWASH19, but CAWASH19 is not active as a standard YR `CanBeOccupied` garrison because the rules flags are commented out.
- Verified gamemd research: `docs/research/GARRISON_OCCUPIED_BUILDING_VISUAL_STATE_GHIDRA_REPORT.md:143..150` states `FUN_00458330` swaps already-live native anim slots and must not be treated as a universal occupied-building visual; it explicitly warns not to assume stock CAWASH19 is a normal garrisoned `ActiveAnimGarrisoned` example.
- Verified gamemd field evidence: `docs/research/BUILDINGTYPECLASS_FIELDS.csv` lists `ActiveAnimGarrisoned` as a BuildingType image field and `CanBeOccupied` defaulting to 0. The active YR report above ties the render/update use to the standard YR building visual path.
- Rust parser evidence: `src/rules/object_type.rs:1034` parses `CanBeOccupied` with default `false`; `src/rules/object_type.rs:1054` parses `MaxNumberOccupants` with default `0`. Since CAWASH19's keys are comments, Rust value is `can_be_occupied=false`, `max_number_occupants=0`.
- Rust art parser evidence: `src/rules/art_data.rs:980..989` stores `ActiveAnimGarrisoned` as `garrisoned_anim_type` on the existing base anim slot, not as a separate overlay.
- Rust spawn evidence: `src/sim/world/world_spawn.rs:405..414` gives garrison cargo only when `obj.can_be_occupied && obj.max_number_occupants > 0`; CAWASH19 has `false && 0`, so its spawned passenger role remains `None`.
- Rust render evidence: `src/app_instances/shp.rs:290` computes `is_garrisoned` from non-empty cargo; CAWASH19 stock spawn has no cargo, so `is_garrisoned=false`. `src/app_instances/shp.rs:529..537` therefore selects `anim.anim_type` (`CAWA19_A`), not `garrisoned_anim_type` (`CAWA19_AG`).

## Stage Results

| Stage | gamemd output | Rust output | Verdict |
|---|---:|---:|---|
| Rules flag parse | `CanBeOccupied=0`, `MaxNumberOccupants=0` for CAWASH19 because the keys are commented/defaulted | `can_be_occupied=false`, `max_number_occupants=0` | PASS |
| Art key parse | `ActiveAnimGarrisoned=CAWA19_AG` is present as an image-field variant, but does not by itself make CAWASH19 garrisonable | `garrisoned_anim_type=Some("CAWA19_AG")` on the `ActiveAnim=CAWA19_A` slot | PASS |
| Standard garrison eligibility | CAWASH19 is not an active `CanBeOccupied` garrison target in standard YR | CAWASH19 gets no garrison cargo at spawn; `is_garrisoned=false` | PASS |
| Building active anim selection | No stock active-garrison CAWASH19 slot switch is reached through the standard `CanBeOccupied` garrison path | `selected_anim_type="CAWA19_A"`, not `"CAWA19_AG"` | PASS |
| Final pixels | Not captured in this trace | Not captured in this trace | UNCHECKED |

## Verdict

Standard YR does not treat stock CAWASH19 as an active `CanBeOccupied` garrison visual case. The current Rust parser/render behavior matches the scoped stock condition: it preserves the `ActiveAnimGarrisoned` art variant in metadata, but stock CAWASH19 is not garrisonable, gets no garrison cargo, and renders the base active anim rather than the garrisoned variant.

This trace finds no stock CAWASH19 FAIL or NOT-IMPLEMENTED finding. Final pixel equality remains UNCHECKED because no native/Rust screenshot or sprite-instance capture was generated in this read-only trace slot.

## Adjacent Findings

- `src/app_instances/shp.rs:529..537` selects a garrisoned active anim from `is_garrisoned` alone and does not directly re-check `can_be_occupied`. This does not trigger for stock CAWASH19 because stock spawn creates no cargo, but a manually constructed impossible entity state could select `CAWA19_AG`. That is outside this scenario.
- `src/app_instances/shp.rs:529..537` prioritizes damaged replacement over garrisoned replacement. Exact combined damaged+garrisoned native priority is not traced here.

## Return Tally

PASS: 4 | FAIL: 0 | UNCHECKED: 1 | NOT-IMPLEMENTED: 0
