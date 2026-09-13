# Saved-seed Load to preview generation

Scope: current-record defaults in RMG saved-seed loading and generation failure ownership.
This is not saved-seed browser or shell parity certification. Button availability,
catalog eligibility, save naming/confirmation, keyboard handling and full visual
comparison remain required work under the [shell acceptance](../../plans/2026-09-12-retail-shells-acceptance.md).

## Native behavior established

Live Ghidra reads on 2026-09-12 target `/gamemd.exe`, image base `0x00400000`,
x86 little endian. Retail executable SHA-256:
`1cdd1180e49024fbda8ad568caac2e86e856063ff67ab38f62b7d2c7bb84298c`.

- `RandomMapSetupDialog__Proc` (`0x00596300`) handles Load `0x6C2` on the
  active Create Random Map dialog. At `0x0059694A` it binds ECX to the live
  MapSeed record `0x00ABDFD8`, then calls the Load modal at `0x0059694F`.
- On success `0x00596963` clears byte `0x0082B030`; `0x0059697C` posts
  `WM_COMMAND/0x620` (Generate). At `0x005969B1` the handler calls
  `SyncControlsFromOptions` (`0x00596E50`) before the posted action runs.
- The Generate branch consumes the latch and calls `SyncOptionsFromControls`
  (`0x00596C70`). When the compared control values remain unchanged, derived fields are not
  rerolled. This condition matters: MapType 0 is absent from the rebuilt combo;
  unlisted theaters are absent too. The custom combo (`0x00617250`) returns
  `-1` for unselected item data, so the subsequent sync marks the record dirty
  and rerolls before clamping. Do not implement an unconditional no-reroll load.
  That sync uses one Size control for both width and height and
  clamps the options. Generate replaces Description with localized
  `TXT_RANDOM_MAP_DESCRIPTION` before running the generator.
- Raw MapSeed load slot `0x00597A30` has instructions but no Ghidra Function
  object. Listing `0x00597B11..0x00597CB2` shows all sixteen integer reads
  passing the current receiver field as the INI default, then storing the
  reader return to that field. Description is different: `0x00597ADE..0x00597B0C`
  passes the localized random-map description default to `0x00528F00`.
  No new function boundary was created.

Reproduce these reads with `get_assembly_context` at `0x00597A30` and
`0x0059694A`, 190 context instructions, and decompile `0x00596300`,
`0x00596C70`, `0x00596E50`. Existing
[saved-seed research](RANDOM_MAP_SAVED_SEED_SLOTS_GHIDRA_REPORT.md) supplies the
active caller/vtable chain; its old implementation-gap list is historical.

## Rust change and validation boundary

`load_saved_seed` now receives the current option record instead of constructing
defaults. The production Load handler supplies its live working options and the
localized Description fallback. It retains its existing invalidation behavior;
automatic preview generation is still missing. Failed file reads leave the
browser and setup intact. A failed worker releases controls
without marking the old or partial preview as a successful map; only successful
completion can enable Save and Use Map.

Regression coverage is recorded in the validation/review packet for the candidate.
The focused loader test exercises a partial seed with all non-default retained
integer fields, clamping, localized and explicit descriptions, and a vanished
file. The failure test exercises old/partial result invalidation, disabled
Save/Accept and a successful retry. This does not demonstrate
native pixel parity or terrain-generator equivalence. Full browser interaction,
native file-parser edge cases and every shell route remain unclosed.

Completing automatic Load-to-Generate requires one owner
for control readback, immediate option-edit derived randomization, the load
latch and Generate; the native map-type `-1` reads preceding range/vector storage
and must not be replaced with an invented clamped bucket.

## Shared Ghidra annotations

The independent critics confirmed the numeric/default identity and the
conditional Load latch behavior from original instructions. The existing
`0x00596E50` plate now records these loading boundaries; the contradictory
unconditional Load-preservation paragraph at `0x00596C70` was corrected.
The `0x00597260` plate also records the raw `-1` argument exception without
guessing predecessor storage. These updates preserved unrelated text, were saved
to `gamemd.exe`, and were
read back exactly. No executable bytes, function boundaries or types changed.

## Rust regression tested (2026-09-12 candidate)

- `cargo test -p vera20k --lib`: 8,686 passed, 0 failed, 121 ignored.
  This includes
  `map::rmg::saved_seeds::tests::partial_seed_preserves_current_integers_and_uses_localized_description_default`
  and
  `ui::skirmish_shell::state::random_map_setup::tests::failed_generation_cannot_accept_or_save_old_or_partial_results`.
- `cargo clippy -p vera20k --lib`: exit 0, 1,147 warnings; no warnings were
  treated as proof of parity or ignored test coverage.
- `git diff --check`: clean. `python -m tools.system_map check`: 0 errors
  across 3 files.

Fresh independent read-only review traced the native defaults and localized
fallback, the sole production loader caller, both worker-failure paths, retained
candidate invalidation, Save/Accept consumers and preview-cache clearing. These
checks support the bounded fixes, not completed RMG behavior or rendered parity.
No new native/Rust frame comparison was performed for this increment.
