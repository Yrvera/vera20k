# Main Menu and Skirmish Shell Parity Repair Design

**Date:** 2026-07-18  
**Status:** Approved by the user; implementation authorized  
**Scope:** Main-menu-to-Skirmish shell lifecycle, offline `[Skirmish]`
persistence, shell/frontend Scenario RNG continuity, Cooperative progress and
country-randomization inputs, accepted-map capacity row behavior, Team defaults,
Random Colour presentation/launch handoff, and per-house offline AI difficulty.  
**Non-scope:** online lobby packet authority, Random Map generation internals,
complete AI-difficulty multiplier coverage, and unverified visual claims from the
disparity scan.

## Goal

Repair the load-bearing visual and functional disparities in the main menu and
offline Skirmish shell while preserving active Yuri's Revenge ordering and
ownership. The shell must retain raw player choices, including Random sentinels,
while a separate launch copy is resolved in the native RNG order. Start and Back
must share the verified pack/randomize/persist transaction, and successful Start
must seed gameplay independently afterward.

The implementation must not claim full shell parity merely because the repaired
surfaces pass. The disparity scan remains the inventory for lower-priority and
unchecked follow-up work.

## Evidence Baseline

Primary evidence:

- `docs/gap-scans/2026-07-18-disparity-scan-main-menu-skirmish-shell.md`
- `docs/research/skirmish-ui/SKIRMISH_RANDOM_COLOR_AND_SETTINGS_PERSISTENCE_TRIGGER_GHIDRA_REPORT.md`
- `docs/research/skirmish-ui/CHOOSE_MAP_ACCEPT_CAPACITY_ROW_SHOW_HIDE_GHIDRA_REPORT.md`
- `docs/research/skirmish-ui/SKIRMISH_MPMODES_MUSTALLY_ALLIESALLOWED_ROW_BEHAVIOR_GHIDRA_REPORT.md`
- `docs/research/skirmish-ui/SKIRMISH_START_GAME_HANDOFF_SESSION_PACKING_GHIDRA_REPORT.md`
- `docs/research/skirmish-ui/SKIRMISH_START_GAME_TO_SPAWN_CONSUMERS_GHIDRA_REPORT.md`
- `docs/research/ADD_TIBERIUM_CREDITS_PURIFIER_VIRTUAL_PURIFIERS_GHIDRA_REPORT.md`
- `docs/research/AI_DIFFICULTY_SYSTEM.md`

Load-bearing verified facts include:

- Start and Back use the same shell pack/randomize path; Start alone validates.
- The raw `[Skirmish]` snapshot is packed before launch assignment resolution and
  retains Random `-2` country/colour values.
- Both paths write `RA2MD.INI` after dialog teardown.
- Shell draws use the process-lifetime Scenario RNG. Successful Start later
  overwrites the gameplay Scenario/main RNG states from a fresh game seed.
- Startup seeds the Scenario RNG before MPModes construction. Stock Cooperative
  progress creation then makes ten logical `(0,2)` map-variant calls before the
  first shell, regardless of the mode later selected.
- Map capacity counts non-`-1` `[Waypoints]` keys `0..7`, falls back to
  `[RandomMap] NumPlayers`, then falls back to `8` when still zero.
- `AlliesAllowed=false` does not disable active Team controls. It changes Team
  defaults; `MustAlly` controls whether `None` is present.
- AI difficulty is a per-house native index: `Hard=0`, `Normal=1`, `Easy=2`.

## Chosen Architecture

The app owns one offline shell runtime:

```text
OfflineSkirmishRuntime
  raw persisted snapshot
  frontend/process Scenario RNG
  Cooperative campaign/progress state
             |
             v
Skirmish shell close transaction
  raw UI state -> resolved launch copy
             |
             v
fresh gameplay seed -> Simulation
             |
             v
normal return captures final Scenario RNG back into OfflineSkirmishRuntime
```

This follows Rust-native ownership while preserving gamemd-native semantics:

- `ui/skirmish_shell/` owns raw controls, visual selection, and input behavior;
- app-level code owns process/session lifecycle, filesystem persistence, and the
  frontend RNG cursor;
- loading receives an already resolved launch session and a separately chosen
  gameplay seed;
- `sim/` owns per-house difficulty and gameplay RNG state; and
- no `sim/` dependency on UI, render, sidebar, audio, or app code is introduced.

## App-Owned Runtime and Close Transaction

`OfflineSkirmishRuntime` holds:

- a durable `SkirmishPersistedSnapshot`;
- the process-continuity `SimRng` used by frontend shell mechanisms; and
- parsed Cooperative campaign/progress state needed by both startup and selected
  Cooperative assignment behavior.

Startup order:

1. Establish the frontend Scenario RNG using the app's boot seed authority.
2. Load/construct MPModes data.
3. Parse `CoopCampMD.ini` and construct source-order Cooperative progress records,
   consuming the verified per-stage `(0,2)` calls after seeding.
4. Read `[Skirmish]` once with native defaults and hydrate raw shell state.

Start/Back close order:

1. Start only: validate. Failure changes no snapshot and consumes no RNG.
2. Pack/clamp `GameMode` and `ScenIndex`.
3. Resolve local Random Country on the frontend Scenario RNG.
4. Pack all AI assignment arrays and seven raw persisted Slot triples.
5. Resolve local Random Colour.
6. Resolve remaining human then all eight AI assignment entries, preserving
   country-before-colour order, collision retries, inactive sentinels, and
   selected-mode country callbacks.
7. Mirror sliders/checkboxes into the durable snapshot.
8. Tear down shell preview/control state.
9. Apply all `[Skirmish]` keys to one in-memory RA2MD.INI buffer and write once.
10. Back discards the resolved launch copy and retains the advanced frontend RNG.
11. Start obtains one fresh gameplay seed and begins loading with the resolved
    launch copy and that seed.

An abnormal shell exit writes the last durable snapshot without fresh packing or
RNG draws. A normal match return copies the final gameplay Scenario RNG into the
frontend runtime before clearing the simulation/startup state.

## Persistence Contract

The snapshot contains ten globals followed by seven Slot triples:

```text
GameMode, ScenIndex, GameSpeed, Credits, UnitCount,
ShortGame, SuperWeaponsAllowed, BuildOffAlly, MCVRepacks, CratesAppear,
Slot01 .. Slot07
```

Slot format is `(row_type_code,country_item_data,colour_item_data)`, with no
spaces. Row codes are `None=1`, `Hard=4`, `Normal=5`, `Easy=6`. Missing Slot01
defaults to `6,-2,-2`; Slot02..07 default to `1,-2,-2`. Booleans use lowercase
`yes/no` and integers use decimal formatting.

Persistence performs one read and one write, preserves unrelated bytes, comments,
sections, duplicate-section policy, and line endings, and treats write failure as
nonfatal because the native caller ignores it.

## Accepted Map Capacity Transaction

One parsed semantic capacity field is carried from map ingestion through
`MapMenuEntry` and `SkirmishScenarioRecord`. No shell consumer derives capacity
from `multiplayer_start_waypoints.len()`.

On accepted Choose Map:

- slot zero remains visible;
- AI row `i` is visible only when `i < capacity - 1`;
- shrinking resets newly hidden rows to inactive/None, releases colour and start
  claims, restores Auto start, applies the selected-mode Team default, and closes
  any hidden-row dropdown/press/drag state;
- growing reveals rows but does not reactivate rows reset by an earlier shrink;
- cancelling Choose Map changes nothing; and
- accepted selection refreshes Team controls/defaults even when the mode is
  unchanged, matching native control reconstruction.

The same visibility predicate gates combo faces, dropdowns, flags, row text,
hit-testing, hover/status help, keyboard/mouse capture, and launch capacity
validation.

## Team and Random Colour Presentation

Team lists/defaults:

- Battle: local `None`, AI `Team D`;
- Team Game: local `Team A`, AI `Team D`;
- Free For All and Cooperative: local and AI `None`;
- `MustAlly=false`: `None,A,B,C,D`;
- `MustAlly=true`: `A,B,C,D`;
- `AlliesAllowed=false` changes defaults but never disables a visible active
  Team control.

Random Colour remains raw in shell state:

- an unclaimed colour selects the Random sentinel, not its cached concrete index;
- the collapsed face and dropdown row draw the verified Random label and no swatch;
- launch packing carries `color_random=true` plus the cached index only as a
  placeholder; and
- resolving a launch copy never mutates the shell selection.

The retail color-sentinel label is `GUI:RandomAsSymbols`: assembly at
`0x004E45EF` loads key pointer `0x00822B7C`; the adjacent `0x20A` immediate is
`GDlgSupp.cpp` source-line metadata, not a string ID. No blank or invented label
may be called parity.

## Cooperative Progress and Country Authority

`CoopCampMD.ini`, not `MPCoopMD.ini`, supplies Cooperative campaigns, three map
variants per stage, and per-stage human/enemy country token lists.

Progress state preserves:

- source-order campaign records;
- chosen variant filename per stage;
- `CampaignType` and `CurrentMap`;
- active progress and per-campaign reserve records; and
- the accepted-campaign active/reserve swap followed by randomized replacement
  initialization.

Country resolution repeatedly draws over the current global country count until
the selected stage's human/enemy list accepts the candidate. Missing campaign or
out-of-range stage returns index zero without a draw. Empty/all-invalid lists have
no parity-safe cap and remain an invalid-content failure risk.

## Per-House AI Difficulty

Add a sim-owned `HouseDifficulty` with native discriminants:

```text
Hard = 0
Normal = 1
Easy = 2
```

`HouseState` defaults to Normal, is serialized and hashed with the field, and is
overwritten for each launched AI from that row's value. `GameOptions` may retain a
global/default difficulty setting but is no longer authoritative for every AI.

The current AIVirtualPurifiers consumer reads the owning `HouseState` difficulty
directly and removes its inverse-index workaround. Other native difficulty
multipliers remain explicit follow-up gaps; this field plumbing does not certify
the entire difficulty system.

Changing `HouseState` serialization requires a coordinated snapshot version bump.

## Expected Code Impact

Likely touchpoints:

- new app/session persistence and runtime modules;
- `src/app.rs`, `src/app_init.rs`, `src/app_skirmish.rs`, `src/app_loading.rs`, and
  match-return handling;
- `src/util/ini_writer.rs` for batched byte-preserving updates;
- `src/skirmish_modes.rs`, `src/skirmish_launch.rs`, and scenario/map metadata;
- `src/ui/skirmish_shell/state/` and `src/app_skirmish_shell_render/`;
- `src/sim/house_state.rs`, world hashing, snapshot versioning, and the miner
  purifier consumer.

Shared code is changed additively and narrowly. No broad UI rewrite, ECS change,
graphics dependency upgrade, or simulation-to-UI dependency is authorized.

## Acceptance Ledger

Persistence/RNG:

- missing-key defaults and exact key/slot formatting;
- byte-preserving one-write update;
- failed Start consumes no RNG and writes nothing;
- identical Start/Back controls consume identical shell transcripts;
- Back/re-enter continues the prior cursor;
- fresh stock startup includes ten logical Cooperative progress calls;
- successful Start's shell draws do not alter gameplay RNG for the same game seed;
- normal match return hands final gameplay Scenario state back to the shell.

Map/visual/input:

- waypoint/RandomMap/fallback capacity fixtures;
- accepted shrink/grow/cancel transactions;
- hidden rows emit no controls, flags, text, hover help, or input targets;
- released hidden colour/start claims reappear in visible dropdowns;
- exact Team lists and refresh defaults without an AlliesAllowed disable gate;
- Random Colour face/dropdown label, no swatch, raw sentinel retained through launch
  preparation.

Difficulty:

- mixed Hard/Normal/Easy rows reach distinct `HouseState` values;
- changing one house difficulty changes the deterministic state hash;
- snapshot round-trip includes the field;
- virtual-purifier indexing uses the native value without inversion.

Verification order:

1. focused parser/state/unit tests;
2. focused shell render/input tests;
3. focused simulation difficulty/snapshot/hash tests;
4. format only edited Rust files with edition 2024 and inspect diffs;
5. one final `cargo check -q` after checking for active Cargo owners;
6. re-scan changed surfaces against the disparity report and list remaining
   `DRIFT`/`UNCHECKED` items without claiming certification.
