# CAMOV02/CASEAT02 Damaged Variant Metadata Postfix Trace

Date: 2026-05-27

Scenario: standard Yuri's Revenge stock `CAMOV02` and `CASEAT02` have `ActiveAnim` plus `ActiveAnimDamaged`. When the building is damaged at or below `ConditionYellow`, the existing active animation slot must switch to `CAMOV02_AD` / `CASEAT02_AD` and consume those damaged variants' own `Start`, `LoopStart`, and `LoopEnd` metadata, not the base `*_A` frame range.

Scope: post-fix verification of damaged ActiveAnim replacement metadata only. This trace does not generalize to every building animation slot, one-shot overlay timing, or the separate native `BuildingClass+0x534` BState modeling gap.

## Evidence

- Stock YR activation evidence: `rulesmd.ini:21716..21719` gives `CAMOV02` `CanBeOccupied=yes`; `rulesmd.ini:16297..16301` gives `CASEAT02` `CanBeOccupied=yes`.
- Stock art evidence: `artmd.ini:7778..7779` gives `CAMOV02 ActiveAnim=CAMOV02_A` and `ActiveAnimDamaged=CAMOV02_AD`; `artmd.ini:11416..11417` gives `CASEAT02 ActiveAnim=CASEAT02_A` and `ActiveAnimDamaged=CASEAT02_AD`.
- Stock damaged metadata:
  - `CAMOV02_AD`: `Start=2`, `LoopStart=2`, `LoopEnd=4`, `LoopCount=-1`, `Rate=100` (`artmd.ini:7801..7809`).
  - `CASEAT02_AD`: `Start=21`, `LoopStart=21`, `LoopEnd=39`, `LoopCount=-1`, `Rate=150` (`artmd.ini:11454..11462`).
- Prior gamemd trace evidence: `CAMOV02_CASEAT02_DAMAGED_ACTIVE_ANIM_SLOT_REPLACEMENT_TRACE.md:78..96` says `CreateAnimForSlot` resolves the selected damaged anim type by name, so the damaged anim consumes its own `Start/LoopStart/LoopEnd`; the previous Rust failure was selecting the damaged key but computing frames from base metadata.
- Current Rust parser: `src/rules/art_data.rs:1005..1028` parses the base anim through `parse_building_anim_variant`, then parses `ActiveAnimDamaged` into `damaged_variant`; `src/rules/art_data.rs:1036..1055` reads `LoopStart`, `LoopEnd`, `LoopCount`, `Rate`, `Start`, and `PingPong` from the named variant section.
- Current Rust render selection: `src/app_instances/shp.rs:518..550` chooses `damaged_variant` when `building_damage_state_active` is true and copies its `anim_type`, `loop_start`, `loop_end`, `loop_count`, `rate`, `start_frame`, and `ping_pong` into the selected view.
- Current Rust frame selection: `src/app_instances/shp.rs:589..628` computes looping active frames from the selected view, not the base `BuildingAnimConfig`.
- Current Rust atlas loading: `src/render/sprite_atlas.rs:633..648` scans damaged/garrisoned variants separately and passes `variant.loop_end`; `src/render/sprite_atlas.rs:1074..1084` includes variant keys in building bounds.

## Pipeline Verdicts

1. Stock data is active in standard YR: PASS.
   - gamemd/INI: both buildings are active stock `CanBeOccupied=yes` civilians with `ActiveAnim` plus `ActiveAnimDamaged`.
   - Rust input data: same `rulesmd.ini` and `artmd.ini` are the repo data source.

2. Damaged variant metadata parse: PASS.
   - gamemd expected metadata by selected anim name:
     - `CAMOV02_AD`: `Start=2`, `LoopStart=2`, `LoopEnd=4`.
     - `CASEAT02_AD`: `Start=21`, `LoopStart=21`, `LoopEnd=39`.
   - Rust source-level calculation:
     - `parse_building_anim_variant("CAMOV02_AD")` reads `Start=2`, `LoopStart=2`, `LoopEnd=4` from `[CAMOV02_AD]`.
     - `parse_building_anim_variant("CASEAT02_AD")` reads `Start=21`, `LoopStart=21`, `LoopEnd=39` from `[CASEAT02_AD]`.

3. Damaged slot selection: PASS.
   - gamemd expected: damaged state selects `CAMOV02_AD` / `CASEAT02_AD` as the active slot replacement.
   - Rust source-level calculation: when `building_damage_state_active == true`, `selected_building_anim_view` returns `anim.damaged_variant`; for these two configs the selected `anim_type` is therefore `CAMOV02_AD` or `CASEAT02_AD`, not the base `*_A`.

4. Looping frame range source: PASS.
   - gamemd expected loop ranges:
     - `CAMOV02_AD`: `2..4` exclusive in the Rust loop model, producing damaged-range frames `2,3`.
     - `CASEAT02_AD`: `21..39` exclusive in the Rust loop model, producing damaged-range frames `21..38`.
   - Rust source-level calculation: `emit_building_anims` calls `looping_frame_values(selected.loop_start, selected.loop_end, selected.rate, selected.ping_pong, ...)`, so the selected damaged variant contributes the frame range. At elapsed `0`, the first damaged frame is `2` for `CAMOV02_AD` and `21` for `CASEAT02_AD`.

5. Atlas availability for damaged frames: PASS.
   - gamemd expected: damaged variants resolve their own named anim type and SHP image (`Image=CAMOV02_A` / `Image=CASEAT02_A`) with damaged frame ranges.
   - Rust source-level calculation: atlas scanning includes `anim.damaged_variant` and uses `variant.loop_end`, so the atlas requests frames through `4` for `CAMOV02_AD` and through `39` for `CASEAT02_AD` rather than only the base ranges `2` and `20`.

6. Exact runtime cadence/tick equality: UNCHECKED.
   - Rust cadence path is visible (`art_rate_to_delay_ms` plus `looping_frame_values`), but this trace did not recompute gamemd's exact wall-clock frame advancement for `Rate=100` or `Rate=150`.
   - This does not reopen the postfix metadata bug; it only means exact frame-timing parity for this active animation cadence remains outside this slot's verified scope.

## Failures

None for the traced postfix metadata mechanic.

## Adjacent Findings

- The render-side damaged-state gate is still health-derived (`building_bstate_damage_active`) rather than a true native `BuildingClass+0x534` BState field. That is adjacent to this trace because the concrete scenario states damage at/below `ConditionYellow`, where the current gate is sufficient to select the damaged variant, but it is not a full model of the native state byte.
- One-shot damaged/garrisoned replacement overlays may still need separate verification because overlay lookup uses the base anim's interned id for active overlay state. `CAMOV02_AD` and `CASEAT02_AD` are infinite loops (`LoopCount=-1`), so this is outside the concrete scenario.

## Tally

PASS: 5 | FAIL: 0 | UNCHECKED: 1 | NOT-IMPLEMENTED: 0

Status: COMPLETE
