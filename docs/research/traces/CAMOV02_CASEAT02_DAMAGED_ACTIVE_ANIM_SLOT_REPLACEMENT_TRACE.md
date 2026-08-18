# CAMOV02/CASEAT02 Damaged ActiveAnim Slot Replacement Trace

Status: PARTIAL - traced stock data, verified docs, and current Rust source; no native/Rust framebuffer capture was produced.

Scenario: standard Yuri's Revenge stock garrisonable civilian buildings `CAMOV02` and `CASEAT02` have `ActiveAnim` plus `ActiveAnimDamaged` in `artmd.ini`. The traced mechanic is whether the damaged active animation replaces the existing active slot instead of drawing as an extra overlay.

## Scope

This trace covers only the damaged `ActiveAnim` replacement behavior for `CAMOV02` and `CASEAT02`.

Adjacent but not traced here:
- `ActiveAnimGarrisoned` occupied healthy replacement.
- Body SHP occupied frame selection.
- Damage fire/smoke overlays.
- Exact `AnimClass` frame timing beyond the first selected damaged frame range mismatch identified below.

## Evidence

- `docs/research/GARRISON_OCCUPIED_BUILDING_VISUAL_STATE_GHIDRA_REPORT.md:88-112`: verified `FUN_00458330 @ 0x00458330` checks existing live slot pointers, then selects replacement strings for slot 3/4/5/6. For slot 3 it selects `Type+0x1018` empty healthy, `Type+0x1038` occupied healthy, or `Type+0x1028` damaged, then calls `BuildingClass::CreateAnimForSlot`.
- `docs/research/BUILDINGCLASS_UPDATE_ANIMATION_GHIDRA_REPORT.md:164-179`: verified `UpdateAnimation` recomputes `GetHealthRatio() > ConditionYellow`; damaged state uses `ConditionYellow` and re-images active slots when the damage threshold is crossed.
- `docs/research/BUILDINGCLASS_UPDATE_ANIMATION_GHIDRA_REPORT.md:415-429`: verified `CreateAnimForSlot` resolves the anim by name, replaces an old anim on slot collision, and `SetAnimSlotImage` chooses damaged art from the damaged per-slot string.
- `ini/artmd.ini:7767-7809`: `CAMOV02` has `ActiveAnim=CAMOV02_A`, `ActiveAnimDamaged=CAMOV02_AD`; damaged anim uses `Image=CAMOV02_A`, `Start=2`, `LoopStart=2`, `LoopEnd=4`.
- `ini/artmd.ini:11409-11464`: `CASEAT02` has `ActiveAnim=CASEAT02_A`, `ActiveAnimDamaged=CASEAT02_AD`; damaged anim uses `Image=CASEAT02_A`, `Start=21`, `LoopStart=21`, `LoopEnd=39`.
- `ini/rulesmd.ini:21716-21719` and `ini/rulesmd.ini:16297-16301`: both target buildings are active stock `CanBeOccupied=yes` civilian garrison buildings.

No live Ghidra MCP call was needed; this trace relies on already verified read-only Ghidra reports.

## Pipeline

`stock INI art/rules` -> `ArtRegistry::parse_building_anims` -> `build_shp_sprite_atlas` loads base and damaged variant keys -> `emit_building_anims` computes damage state and selected anim type -> frame key lookup -> screen overlay.

## Stage Verdicts

### Stage 1 - Stock data identity

Gamemd input:
- `CAMOV02` active slot 3 strings: healthy `CAMOV02_A`, damaged `CAMOV02_AD`.
- `CASEAT02` active slot 3 strings: healthy `CASEAT02_A`, damaged `CASEAT02_AD`.

Rust input:
- `src/rules/art_data.rs:980-990` stores one `BuildingAnimConfig` with `anim_type`, `damaged_anim_type`, and `garrisoned_anim_type`.
- `src/rules/art_data_tests.rs:224-247` covers one active anim with damaged/garrisoned variants attached to one config.

Verdict: PASS for parsed string identity and one-config-per-base-slot shape.

### Stage 2 - Replacement versus extra overlay

Gamemd:
- Slot 3 is a single live `AnimClass*` at `Building+0x568`; `CreateAnimForSlot` replaces the prior slot object on collision rather than creating a second slot.
- `FUN_00458330` only acts if the slot pointer already exists, so the damaged variant is a replacement of slot 3.

Rust:
- `src/app_instances/shp.rs:523-537` iterates one config and chooses exactly one `selected_anim_type`.
- `src/app_instances/shp.rs:593-607` performs one atlas lookup and emits one `SpriteInstance` for that config.

Concrete count:
- Gamemd damaged `CAMOV02`/`CASEAT02` active overlays for this mechanic: 1 live slot.
- Rust damaged `CAMOV02`/`CASEAT02` active overlays for this config: 1 emitted overlay.

Verdict: PASS for overlay count and replacement/selection shape.

### Stage 3 - Damage threshold selection

Gamemd:
- Damaged active art is selected when health ratio is at or below `ConditionYellow`; default `ConditionYellow=0.5`.

Rust:
- `src/app_instances/shp.rs:292-300` computes `building_damage_state_active`.
- `src/app_instances/shp.rs:691-700` returns `(health_current as f32 / health_max as f32) <= condition_yellow`.
- `src/app_instances/shp.rs:529-537` selects `damaged_anim_type` before garrisoned or base when the flag is true.

Concrete check:
- At exactly 50% health, gamemd damaged selector is true.
- At exactly 50% health, Rust selector is true.

Verdict: PASS for this concrete threshold sample. Full fixed/double/f32 equivalence across all health/max pairs remains UNCHECKED.

### Stage 4 - Damaged anim frame metadata

Gamemd:
- `CreateAnimForSlot` resolves the selected damaged anim type by name. Therefore `CAMOV02_AD` consumes its own `Start=2`, `LoopStart=2`, `LoopEnd=4`; `CASEAT02_AD` consumes its own `Start=21`, `LoopStart=21`, `LoopEnd=39`.

Rust:
- `src/rules/art_data.rs:962-978` reads loop/rate/start metadata only from the base `anim_type` section before attaching `damaged_anim_type`.
- `src/app_instances/shp.rs:529-537` switches the SHP key to the damaged anim type, but `src/app_instances/shp.rs:541-575` still computes the frame from the base `BuildingAnimConfig`.
- `src/app_instances/shp.rs:468-489` uses `anim.loop_start`, `anim.loop_end`, and `anim.rate` from that base config.

Concrete mismatch:
- `CAMOV02_A` base loop is `0..2`; `CAMOV02_AD` damaged loop starts at `2`. Rust selects key `CAMOV02_AD` but computes frames from base loop start `0`.
- `CASEAT02_A` base loop is `0..20`; `CASEAT02_AD` damaged loop starts at `21`. Rust selects key `CASEAT02_AD` but computes frames from base loop start `0`.

Player-visible result:
- The overlay count is correct, but the damaged replacement can display healthy-range frames from the shared SHP instead of the damaged-range frames. On the target buildings this means the player can see the active animation fail to switch to its damaged frame range when the building crosses yellow health.

Verdict: FAIL.

### Stage 5 - Atlas availability for replacement SHP keys

Rust:
- `src/render/sprite_atlas.rs:633-668` loads `damaged_anim_type` and `garrisoned_anim_type` variant keys.
- `src/render/sprite_atlas.rs:38-63` resolves the variant effective image id before scanning frame count.
- `src/render/sprite_atlas.rs:1065-1085` includes variant keys in building bounds.

Gamemd:
- Verified reports say `CreateAnimForSlot` resolves the selected anim by name, so the damaged variant must be resolvable by name.

Verdict: PASS for key availability shape. Pixel equality remains UNCHECKED because this trace did not compare rendered native and Rust frames.

## Failures

1. Stage 4 - Damaged variant frame metadata is not used by current Rust.
   - Rust selects `CAMOV02_AD`/`CASEAT02_AD` as a replacement key but keeps frame timing from `CAMOV02_A`/`CASEAT02_A`.
   - Gamemd creates/re-images the slot using the damaged anim type, so the damaged anim's own `Start`/`LoopStart`/`LoopEnd` apply.
   - Visible difference: the replacement slot count is correct, but the wrong frame range can be shown after damage.

## Not Implemented

None for the narrow "extra overlay versus replacement slot" question. The incorrect damaged frame metadata is implemented incorrectly, not missing.

## Timing

- Replacement timing at the exact health-crossing tick was not recomputed from live code in this trace. The verified docs state `SetDamagedState`/`CreateAnimForSlot` re-images active slots when the damage threshold changes.
- Rust render selection is stateless per frame and tied to `building_damage_state_active`; exact tick parity is UNCHECKED.

## Verdict Tally

PASS: 4
FAIL: 1
UNCHECKED: 2
NOT-IMPLEMENTED: 0

