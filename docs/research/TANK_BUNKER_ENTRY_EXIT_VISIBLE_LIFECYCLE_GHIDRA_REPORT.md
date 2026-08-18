# Tank Bunker Entry/Exit Visible Lifecycle - Ghidra Research Report

Date: 2026-05-23

Target question: For the stock `NATBNK` tank bunker path, what visible lifecycle does gamemd.exe run for entry admission, install/hide, exit/release, sell/destruction cleanup, wall sounds, state flags, and reciprocal `BuildingClass`/`TechnoClass +0x2E4` links?
Non-goals: Refinery unload, unit repair depots, civilian `CanBeOccupied` garrisons, full `NumberImpassableRows` row-helper semantics, and bunker combat multipliers except where they prove the `+0x2E4` visible lifecycle.
Evidence needed to mark COMPLETE: INI/default evidence for stock activation and sounds; decompile plus xref/caller evidence for radio entry, install, release, sell/destruction/clear helpers; current Rust scan; implementation handoff with acceptance scenarios.
Stop conditions: Stop once the stock `NATBNK` visible entry/install/exit/cleanup lifecycle is closed, or record missing helper boundaries/path liveness as Remaining Uncertainty; do not expand into refinery, repair depot, or civilian garrison systems.

**Address(es):** `0x0043C2D0`, `0x00458E50`, `0x004595C0`, `0x004593A0`, `0x00459470`, `0x0073D630`, `0x0070FB50`, `0x00669E20`
**Investigation Mode:** exhaustive-slice
**Claimed Scope:** Stock `NATBNK` `Bunker=yes` unit entry admission, install, normal unit deploy exit, sell/destruction/temporal/super clear paths, visible wall sounds/animations, and reciprocal `+0x2E4` link lifetime.
**Non-Scope:** Refinery dock/unload, UnitRepair depot dock, civilian garrison/passenger vector, exact pathfinding row-helper effects, bunker combat damage/ROF/range math.
**Confidence:** High for the claimed lifecycle.
**Active in YR:** Conditional. Active for buildings whose `BuildingTypeClass+0x16AB` is set by `Bunker=yes`; checked stock `rulesmd.ini` sets this for `[NATBNK]`.

## 1. Overview

The tank bunker lifecycle is a single-slot reciprocal link, not a passenger cargo or civilian garrison vector. `BuildingClass+0x2E4` stores the installed unit, and the unit's `TechnoClass/FootClass+0x2E4` stores the containing building; install writes both, and all verified exit/clear paths clear both.

Player-visible side effects are wall up/down sounds, entry/exit animation slots selected by building health, hiding/limboing the installed unit, and reappearing or clearing state through different helpers depending on whether the unit is normally released or the building/unit is being cleaned up.

## 2. Class Layout / Key Offsets

| Owner | Offset | Meaning | Evidence | Active in YR |
|---|---:|---|---|---|
| `BuildingTypeClass` | `+0x16AB` | `Bunker=yes` type flag | `BuildingClass__Receive_Radio` checks `Type[0x16AB]`; `MissionRepairAndProduce` calls `0x00458E50` only after `MOV CL,[EAX+0x16AB]`, `TEST`, `CALL 0x00458E50` at `0x0044B797..0x0044B7A3`; prior parser proof `BuildingTypeClass::ReadINI @ 0x00460941..0x00460954` | Yes for stock `[NATBNK]`; No for checked stock `[NABNKR]` without an override |
| `TechnoTypeClass` | `+0xD2E` | `Bunkerable` admission flag | `TechnoClass__CanAutoDeployHere @ 0x0070FB50` reads `type+0xD2E`; prior constructor/default proof: `UnitTypeClass` writes true at `0x007472AA`, non-unit techno types default false | Yes for standard units unless overridden |
| `RulesClass` | `+0x240` | `BunkerWallsUpSound` resolved sound id | `RulesClass__ReadAudioVisual @ 0x00669E20` writes `param_1[0x90]`; install state checks `g_RulesClass+0x240 != -1` and calls `VocClass__PlayAt` at `0x0045933D..0x0045936F` | Yes when configured; stock value `TankBunkerUp` |
| `RulesClass` | `+0x244` | `BunkerWallsDownSound` resolved sound id | `RulesClass__ReadAudioVisual @ 0x00669E20` writes `param_1[0x91]`; release/clear helpers check `g_RulesClass+0x244 != -1` | Yes when configured; stock value `TankBunkerDown` |
| `BuildingClass` | `+0x2E4` | Single installed unit pointer | Install write `0x00459301`; release/clear reads and clears | Conditional while occupied |
| `TechnoClass` / unit | `+0x2E4` | Reciprocal containing bunker pointer | Install write `piVar5[0xB9] = building` at `0x0045930F`; release/clear writes zero to unit side | Conditional while inside bunker |
| `BuildingClass` | `+0x718` | Bunker state-machine state | State switch source in `0x00458E50`; install sets `6`, clear/release resets `0` | Conditional for `Bunker=yes` mission helper |
| Unit object | `+0x214` | Companion field cleared on install | Install writes `piVar5[0x85] = -1` at `0x00459315` | Conditional during install |

## 3. Core Logic

### Entry Admission

| Finding | Evidence | Confidence | Active in YR |
|---|---|---:|---|
| Radio case `0x0F` accepts only allied/powered/eligible targets and reaches a bunker-specific branch only when `Type[0x16AB] != 0`. | `BuildingClass__Receive_Radio @ 0x0043C2D0`, case `0x0F`; decompile branch `if (puVar8[0x16AB] != 0)` | High | Conditional: target building has `Bunker=yes` |
| The bunker `0x0F` branch calls `TechnoClass__CanAutoDeployHere`; false returns `10`, a successful `vtable+0x278(0x23, sender)` contact check returning `1` also returns `10`, otherwise returns `1`. | Decompile `0x0043C2D0`; xrefs to `TechnoClass__CanAutoDeployHere` include `0x0043C512`; function xref evidence | High | Conditional: allied, powered, live entry request |
| `CanAutoDeployHere` gates on `Bunkerable`, deploy compatibility, weapon/turret availability, movement-zone exclusion, and a runtime flag/reference guard. | `TechnoClass__CanAutoDeployHere @ 0x0070FB50`: reads `type+0xD2E`, `type+0xCA1`, `vtable+0x3F4`, `type+0x67C != 3`, and `this->field_0x14 & 4` guard | High | Yes for the stock bunker entry path |
| Radio case `0x15` is the visible handoff into the building mission helper: for `Bunker=yes`, it writes `field_0x6DD = 1`, calls `MissionSet(0x14,0)`, and returns `1`. | `BuildingClass__Receive_Radio @ 0x0043C2D0`, case `0x15`, `Type[0x16AB]` branch | High | Conditional: sender arrives/completes radio handoff |

### Install State Machine

| State / branch | Behavior | Evidence | Active in YR |
|---|---|---|---|
| Mission integration | `MissionRepairAndProduce` calls the bunker helper before other sibling helpers when `BuildingType+0x16AB` is true. | Xref `0x0044B7A3 -> 0x00458E50`; assembly context `MOV [EBP+0x520]`, `MOV CL,[EAX+0x16AB]`, `TEST`, `CALL 0x00458E50` | Conditional: `Bunker=yes` building on mission `0x14` |
| Preflight | Helper reads `Building+0x2E4`; if empty, falls back to `FootClass__GetDestination(0)`. If no candidate or candidate `WhatAmI()!=1`, it resets `+0x718=0` and missions the building to `5`. | `buildingclass_bunker_occupant_dock_link_writer @ 0x00458E50` decompile | Conditional |
| State 0/1/2/3 | The unit must be at the building cell and not stopped by locomotor status; the helper scans foundation offsets, shoves/handles nearby blockers, computes facing, waits timers, and calls locomotor/head and unit `vtable+0x544(0,0x3FF00000)`. | `0x00458E50` decompile states `0..3` | Conditional; visible through approach/entry cadence but row-helper details are non-scope |
| State 4 | Entry animations are health-gated: healthy uses `BuildingType+0x11F4/+0x1238`, red/damaged uses `+0x1204/+0x1248`; each nonempty slot calls `BuildingClass__CreateAnimForSlot`. | `0x00458E50` state `4`; threshold `RulesClass+0x1700 ConditionRed` | Conditional: art slots configured |
| State 5 install | Writes `building+0x2E4 = unit`, `unit+0x2E4 = building`, `unit+0x214 = -1`; calls unit `vtable+0x150`; writes `building+0x718=6`; missions the unit with `(5,1)`. | `0x00459301..0x00459337` decompile and disassembly range check | Conditional: successful state-machine install |
| Wall-up sound | Install plays `RulesClass+0x240` at building location if it is not `-1`. | `0x0045933D..0x0045936F`; `rulesmd.ini:719` | Yes for stock `TankBunkerUp` |

### Exit / Release / Cleanup

| Path | Behavior | Evidence | Active in YR |
|---|---|---|---|
| Normal unit deploy exit | `UnitClass__Mission_Deploy_Building` checks `unit+0x2E4 != 0`, looks up the building in the current cell, and calls `BuildingClass__ReleaseDockedHarvester`. | `UnitClass__Mission_Deploy_Building @ 0x0073D630`; call xref `0x0073D66D -> 0x004595C0`; assembly context confirms call after lookup | Conditional: bunkered unit is told to deploy/exit |
| Full release helper | Clears anim slots `10` and `11`, plays `RulesClass+0x244` if configured, creates health-gated exit/down animations from `BuildingType+0x127C/+0x128C` and `+0x12C0/+0x12D0`, clears unit back-link, stops/heads locomotor with track/facing argument `0x47` and offset `(-0x80,+0x80)`, applies unit `vtable+0x544(0,0x3FF00000)`, finds a nearby passable cell from building anchor `(-1,+1)`, places the unit, missions unit to `2`, clears building link and state, missions building to `5`, and sends radio `3`. | `BuildingClass__ReleaseDockedHarvester @ 0x004595C0`; decompile plus xref `0x0073D66D`; assembly ranges `0x004595C0..0x004597BF` checked | Conditional: normal full exit |
| Sell/destruction/temporal undock helper | `BuildingClass__UndockUnit` gets building-side `+0x2E4`, requires candidate `WhatAmI()==1`, stops/heads locomotor with `0x47` and offset `(-0x80,+0x80)`, applies `vtable+0x544`, clears unit and building `+0x2E4`, and sends radio `3`; it does not run the full nearby-passable-cell placement sequence. | `0x004593A0` decompile; xrefs `BuildingClass__Sell @ 0x0044AAB0`, `BuildingClass__ReceiveDamage @ 0x004424EA`, `TemporalClass__Update @ 0x0071AA15`; assembly context for sell reads `[EBP+0x2E4]`, tests, then calls | Conditional: occupied building being sold/destroyed/temporal path |
| Clear-only helper | `FUN_00459470` clears anim slots, and only if `building+0x2E4 != 0` plays down sound, creates exit/down animations, sends radio `3`, clears unit back-link, clears building link and `+0x718`, and missions building to `5`; it does not unhide/place/mission the unit. | `0x00459470` decompile; xrefs `SuperClass__Launch @ 0x006CC955`, `TemporalClass__Update @ 0x0071AA90`, `UnitClass__ReceiveDamage @ 0x00737D97` | Conditional: super/temporal/unit-damage edge clear |

## 4. INI Keys

| Section | Key | Stock YR value | Effect | Evidence | Active in YR |
|---|---|---:|---|---|---|
| `[AudioVisual]` | `BunkerWallsUpSound` | `TankBunkerUp` | Parsed to `RulesClass+0x240`; played once on successful install if sound id is not `-1`. | `rulesmd.ini:719`; `RulesClass__ReadAudioVisual @ 0x00669E20`; install `0x0045933D..0x0045936F` | Yes |
| `[AudioVisual]` | `BunkerWallsDownSound` | `TankBunkerDown` | Parsed to `RulesClass+0x244`; played by normal release and clear-only helpers when occupied/configured. | `rulesmd.ini:720`; `RulesClass__ReadAudioVisual @ 0x00669E20`; release `0x004595C0`, clear `0x00459470` | Yes |
| `[NATBNK]` | `Bunker` | `yes` | Sets `BuildingType+0x16AB`, enabling entry, mission helper, and lifecycle. | `rulesmd.ini:13722`, `13732`; binary gates above | Yes |
| `[NATBNK]` | `NumberOfDocks` | `1` | Supports dock-style admission/state setup; full dock semantics are non-scope. | `rulesmd.ini:13750`; prior row-helper report | Conditional; not sufficient without `Bunker=yes` |
| `[NATBNK]` | `NumberImpassableRows` | `0` | Affects pathing helper; non-scope except occupancy state is `+0x2E4`. | `rulesmd.ini:13751`; prior row-helper report | Yes |
| Unit sections | `Bunkerable` | unit default true, many stock overrides `no` | Gates `CanAutoDeployHere`; non-unit techno types default false. | `TechnoClass__CanAutoDeployHere @ 0x0070FB50`; prior constructor/default proof; `rulesmd.ini` override lines such as `7008` | Yes |
| `[NABNKR]` | `Bunker` | absent in checked stock section | Does not activate this lifecycle unless modded/overridden. | `rulesmd.ini:12979` section lacks `Bunker=yes` in checked data | No for checked stock lifecycle |

## 5. Integration Points

Entry is radio-driven: `BuildingClass__Receive_Radio` case `0x0F` validates the sender and case `0x15` sets the building to mission `0x14`, after which `MissionRepairAndProduce` reaches `0x00458E50` only for `Bunker=yes` buildings. This is active in YR for stock `NATBNK`; no `SpecialFlags` or TS-only gate was found on the claimed path.

Exit is unit-driven for normal release: a unit with `unit+0x2E4 != 0` in `UnitClass__Mission_Deploy_Building` calls `ReleaseDockedHarvester` via the current-cell building lookup. Sell/destruction/temporal/super/unit-damage cleanup call separate helpers and must not be collapsed into one generic garrison-eject path.

## 6. Current Rust Implementation Status

Rust now has more of the data surface than older reports recorded:

| Rust area | Current status | Evidence | Remaining delta |
|---|---|---|---|
| `Bunker=yes` parsing | Present | `src/rules/object_type.rs:654`, `src/rules/object_type.rs:1085`, test `bunker_flag_parses_from_ini` | none for the flag |
| `Bunkerable` parsing/defaults | Present | `src/rules/object_type.rs:614`, `src/rules/object_type.rs:1065`, tests `bunkerable_defaults_true_for_vehicles_only`, `bunkerable_ini_overrides_vehicle_default` | admission must use it in the bunker lifecycle, not only data |
| Building-side occupant | Partial | `src/sim/game_entity.rs:276` `bunker_occupant`; movement occupancy reads it | no verified install/release state machine writes it yet |
| Unit-side back-reference | Missing as a bunker-specific lifecycle invariant | Rust scan found `PassengerRole::Inside` for transports/garrisons but no separate bunker back-reference | needed for combat/exit/link cleanup parity |
| Movement row-helper occupancy | Partial | `src/sim/movement/movement_occupancy.rs:191`, `src/sim/pathfinding/cell_entry.rs:188` | current fallback also treats passenger cargo as occupied; lifecycle must keep bunker separate from garrison cargo |
| Sounds | Partial/stale naming | `RulesGeneral.bunker_walls_down_sound` at `src/rules/ruleset.rs:267`; parsed at `:902`; `RefineryExitSfx` app/audio comments mention bunker down | missing `BunkerWallsUpSound` parse/event and dedicated bunker wall sound event names |
| Entry/install/exit/clear lifecycle | Missing | `rg` found no bunker mission/radio state machine surface | implement entry admission, install hide/link, full release, and cleanup helpers |

No Rust, INI, or in-repo doc files were modified during this investigation.

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| Stock `NATBNK` activation | verified | `rulesmd.ini:13722`, `13732`; `BuildingType+0x16AB` gates | none |
| `NABNKR` negative separation | verified | checked `rulesmd.ini:12979` lacks `Bunker=yes` | none unless a mod override is investigated |
| `Bunkerable` binary gate | verified | `TechnoClass__CanAutoDeployHere @ 0x0070FB50`; prior default proof | none for gate/default |
| Radio case `0x0F` admission | verified | `BuildingClass__Receive_Radio @ 0x0043C2D0`, xref `0x0043C512` | exact user command source not traced; not needed for lifecycle |
| Radio case `0x15` handoff | verified | `0x0043C2D0` case `0x15`, `Type[0x16AB]` branch | none |
| Mission helper call | verified | xref `0x0044B7A3 -> 0x00458E50`, assembly context | none |
| Install states and wall-up sound | verified | `0x00458E50`, `0x00459301..0x0045936F` | exact vtable method names not required |
| Normal exit release | verified | `0x0073D630`, xref `0x0073D66D -> 0x004595C0`; `0x004595C0` | none for visible lifecycle |
| Sell/destruction/temporal undock | verified | `0x004593A0` plus xrefs `0x0044AAB0`, `0x004424EA`, `0x0071AA15` | none for link cleanup |
| Clear-only helper | verified | `0x00459470` plus xrefs `0x006CC955`, `0x0071AA90`, `0x00737D97` | exact super/temporal scenario semantics are non-scope |
| Civilian garrison separation | verified by negative comparison | this report plus garrison reports; bunker uses single reciprocal `+0x2E4`, garrison uses `PassengerCargo`/occupant vector | none |

## 8. Open Questions - Final State

- `[RESOLVED] OQ-1 - What activates the stock path? -> checked stock [NATBNK] has Bunker=yes; [NABNKR] does not.` (evidence: `rulesmd.ini:13722`, `13732`, `12979`; `BuildingType+0x16AB` gates)
- `[RESOLVED] OQ-2 - What admits a unit? -> radio case 0x0F uses Bunker=yes plus CanAutoDeployHere, including Bunkerable at TechnoType+0xD2E.` (evidence: `0x0043C2D0`, `0x0070FB50`)
- `[RESOLVED] OQ-3 - What starts install? -> radio case 0x15 sets mission 0x14 for Bunker=yes buildings, and MissionRepairAndProduce calls 0x00458E50.` (evidence: `0x0043C2D0`, `0x0044B7A3`)
- `[RESOLVED] OQ-4 - What writes reciprocal links? -> install state 5 writes building+0x2E4 and unit+0x2E4.` (evidence: `0x00459301..0x0045930F`)
- `[RESOLVED] OQ-5 - What visible install sound plays? -> RulesClass+0x240 BunkerWallsUpSound at building location if not -1.` (evidence: `0x0045933D..0x0045936F`; `rulesmd.ini:719`)
- `[RESOLVED] OQ-6 - What is normal exit? -> bunkered unit deploy calls 0x004595C0, which plays down sound, creates exit/down anims, clears links, places unit nearby, and resets state.` (evidence: `0x0073D66D`, `0x004595C0`)
- `[RESOLVED] OQ-7 - What do sell/destruction paths use? -> 0x004593A0 undocks/clears links without the full nearby-passable-cell sequence.` (evidence: xrefs `0x0044AAB0`, `0x004424EA`, `0x0071AA15`; decompile `0x004593A0`)
- `[RESOLVED] OQ-8 - What do super/temporal/unit-damage clear paths use? -> 0x00459470 clear-only helper clears both links/state and plays down anim/sound only if occupied.` (evidence: xrefs `0x006CC955`, `0x0071AA90`, `0x00737D97`; decompile `0x00459470`)
- `[RESOLVED] OQ-9 - Is this civilian garrison/passenger cargo? -> no, the lifecycle uses single reciprocal +0x2E4 links and no occupant vector.` (evidence: install/release writes above; garrison reports for contrast)
- `[RESOLVED] OQ-10 - What exists in current Rust? -> data flags and building-side occupant exist; lifecycle/back-reference/up sound are missing.` (evidence: Rust scan paths in Section 6)

## 9. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| Entry admission requires `Bunker=yes` building and `CanAutoDeployHere`-eligible unit, including `Bunkerable`; stock `NATBNK` qualifies and checked `NABNKR` does not. | `0x0043C2D0`, `0x0070FB50`, `rulesmd.ini:13732`, `12979` | flags parsed, lifecycle admission missing | command/radio/mission entry planning; rules object data | Reject infantry/non-bunkerable/stock `NABNKR`; accept eligible vehicle into `NATBNK` handoff | Player orders a default tank into `NATBNK`: accepted; infantry and `Bunkerable=no` unit reject; stock `NABNKR` rejects. Proposed test: `bunker_entry_requires_bunker_flag_and_bunkerable_vehicle` | Do not string-special-case NATBNK or make all techno types bunkerable |
| Install writes building and unit reciprocal `+0x2E4`, clears unit `+0x214`, hides/limbos the unit, sets state `6`, missions unit to `5`, and plays `BunkerWallsUpSound` once. | `0x00459301..0x0045936F`; `rulesmd.ini:719` | building-side field exists; unit back-reference/hide/up sound/state machine missing | `src/sim/game_entity.rs`, bunker mission system, sound events/rules parsing | Maintain a two-sided bunker link separate from `PassengerRole`, remove installed unit from normal map occupancy/visibility as gamemd does, and emit `TankBunkerUp` | After install, building has occupant id, unit has bunker back-reference/is hidden, no passenger cargo is created, and one up sound event exists. Proposed test: `bunker_install_hides_unit_sets_reciprocal_links_and_emits_up_sound` | High: one-sided links create stuck units or combat/exit mismatches |
| Normal exit and cleanup paths are distinct: unit deploy exit uses full release/place; sell/destruction uses undock-clear; clear-only helper clears links/state and plays down anim/sound without full placement. | `0x0073D66D -> 0x004595C0`; `0x004593A0` xrefs; `0x00459470` xrefs; `rulesmd.ini:720` | lifecycle missing; down sound currently has stale `RefineryExitSfx` naming | building sell/damage/destruction, unit deploy, world cleanup, sound event mapping | Implement separate full-release vs clear-only/undock paths; always clear both links and building state; emit dedicated bunker down event where binary does | Exiting an occupied bunker places/reveals the unit and clears both links; selling/destroying occupied bunker clears both links without civilian garrison ejection; down sound emits once. Proposed test: `bunker_exit_sell_destroy_clear_links_with_distinct_release_modes` | High: do not reuse civilian garrison ejection or refinery unload sound semantics |

### Stale Docs / Follow-up Docs

- `C:/Users/enok/Documents/ra2-rust-game-docs/BUNKER_SYSTEM_GHIDRA_REPORT.md`: Replace Rust-status wording claiming `Bunker=yes`, `Bunkerable`, or `BuildingClass+0x2E4` equivalents are not parsed/stored with: "Current Rust parses `Bunker` into `ObjectType.bunker`, parses `Bunkerable` with unit-default true/non-unit-default false into `ObjectType.bunkerable`, and stores a building-side `GameEntity.bunker_occupant`; remaining gaps are bunker radio/mission admission, install hide/link semantics including unit back-reference, normal release vs clear-only lifecycle, `BunkerWallsUpSound`, and dedicated bunker wall sound events."
- `C:/Users/enok/Documents/ra2-rust-game-docs/BUNKER_SYSTEM_GHIDRA_REPORT.md`: Replace wording implying stock `NABNKR` is a verified `Bunker=yes` lifecycle user with: "Checked stock `rulesmd.ini` sets `Bunker=yes` on `[NATBNK]`; checked `[NABNKR]` is live content but does not set `Bunker=yes`, so it is not proven to enter the tank-bunker lifecycle without an override."
- `C:/Users/enok/Documents/ra2-rust-game-docs/BUNKER_0X2E4_LIFECYCLE_EXIT_CLEAR_PATH_GHIDRA_REPORT.md`: Replace Rust-status wording saying there is no `Bunkerable` parser/default surface with: "Current Rust parses `Bunkerable` with unit-default true and non-unit-default false; the remaining gap is using that data in the radio/mission bunker lifecycle."

## Negative Facts / Do Not Do

- Do not implement stock tank bunkers as `PassengerRole::Transport` or civilian `CanBeOccupied` garrison cargo. Active in YR: Yes; evidence is reciprocal `+0x2E4` writes at `0x00459301/0x0045930F`.
- Do not use `NABNKR` as the stock checked `Bunker=yes` acceptance fixture. Active in YR: No for this lifecycle in checked data; evidence `rulesmd.ini:12979` lacks `Bunker=yes`.
- Do not collapse full release, undock, and clear-only helpers into one behavior. Active in YR: Conditional; evidence separate helpers `0x004595C0`, `0x004593A0`, `0x00459470` with different placement/unhide behavior.
- Do not model `BunkerWallsDownSound` as normal refinery unload audio. Active in YR: Yes for bunker release/clear only; evidence release/clear helpers read `RulesClass+0x244`, while stock zero-link refinery unload path does not set reciprocal `+0x2E4`.
- Do not clear only one side of the link. Active in YR: Yes; evidence install writes both sides and all verified cleanup paths clear both sides.

## Remaining Uncertainty

None for the scoped visible lifecycle. Exact legacy names for unit `vtable+0x150`, unit `vtable+0x544`, and locomotor `+0x58/+0x70` are inferred from call shape, but the report's required Rust behavior is based on verified writes/calls/order, not the names.

## Sources

- Ghidra decompile/read-only: `BuildingClass__Receive_Radio @ 0x0043C2D0`; `TechnoClass__CanAutoDeployHere @ 0x0070FB50`; `buildingclass_bunker_occupant_dock_link_writer @ 0x00458E50`; `BuildingClass__ReleaseDockedHarvester @ 0x004595C0`; `BuildingClass__UndockUnit @ 0x004593A0`; `FUN_00459470`; `UnitClass__Mission_Deploy_Building @ 0x0073D630`; `RulesClass__ReadAudioVisual @ 0x00669E20`.
- Ghidra xrefs: `0x0044B7A3 -> 0x00458E50`; `0x0073D66D -> 0x004595C0`; `0x0044AAB0`, `0x004424EA`, `0x0071AA15 -> 0x004593A0`; `0x006CC955`, `0x0071AA90`, `0x00737D97 -> 0x00459470`; `0x0043C512`, `0x0043C86A -> 0x0070FB50`.
- INI/art checked: `ini/rulesmd.ini:719`, `720`, `12979`, `13722`, `13732`, `13750`, `13751`; `ini/artmd.ini:5019`.
- Prior reports checked: `BUNKER_0X2E4_LIFECYCLE_EXIT_CLEAR_PATH_GHIDRA_REPORT.md`; `NATBNK_BUNKER_0X2E4_ROW_HELPER_PATH_GHIDRA_REPORT.md`; `BUNKER_SYSTEM_GHIDRA_REPORT.md`; `GARRISON_OCCUPANT_DEATH_REMOVAL_PENETRATESBUNKER_GHIDRA_REPORT.md`.
- Rust scan checked: `src/rules/object_type.rs`; `src/rules/ruleset.rs`; `src/sim/game_entity.rs`; `src/sim/movement/movement_occupancy.rs`; `src/sim/pathfinding/cell_entry.rs`; `src/audio/events.rs`; `src/app_sim_tick.rs`.
