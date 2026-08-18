# Docking Queue Exit Reference Points - Ghidra Research Report

**Address(es):** `0x0045FE50`, `0x0043B740`, `0x00447B20`, `0x0043C2D0`, `0x0073E5E0`, `0x0073D630`, `0x004595C0`, `0x00443C60`, `0x0044F640`, `0x0044EFB0`, `0x0065AD90`, `0x0065ADF0`, `0x0065AE60`  
**Investigation Mode:** exhaustive-slice  
**Claimed Scope:** Docking and production reference points for `QueueingCell`, `DockingOffset%d`, `ExitCoord`, `NumberOfDocks`, stock refinery accepted/wait/exit cells, stock land war-factory spawn cell, airfield/depot-style pad coordinates, and contact-slot relationship.  
**Non-Scope:** full ore economy, exact same-frame two-miner promotion order, full aircraft flight/reload FSM, naval yard exit behavior, service depot repair cadence, and full production queue refund/retry policy.  
**Confidence:** High for parser units, stock refinery cells, conditional release cells, `NumberOfDocks`/contact-slot relationship, and stock land war-factory `ExitCoord`; Medium for all non-stock mod combinations because runtime mission policy can gate which helper is reached.  
**Active in YR:** Yes for stock `CMIN/HARV -> GAREFN/NAREFN`, stock `GAWEAP/NAWEAP/YAWEAP`, stock `GAAIRC/AMRADR` aircraft pads, and stock depot pad parsing. Conditional for `ReleaseDockedHarvester` / `UndockUnit` reciprocal-link exits.

## 0. Investigation Contract

**Target question:** What are the exact coordinate reference points and units for `QueueingCell`, `DockingOffset%d`, `ExitCoord`, and `NumberOfDocks`, and which of them drive refinery docking, refinery queue/wait staging, stock unload exit, war-factory production exit, and direction-dependent approach/release cells?

**Non-goals:** Do not implement Rust. Do not re-cover full miner unload credit timing, full airfield reload behavior, naval production exits, or service-depot repair behavior except where needed to distinguish reference-point contracts.

**Evidence needed to mark COMPLETE:** INI parser evidence for field units/defaults/storage; fresh binary evidence for live consumers; stock INI evidence for YR liveness; explicit stock-vs-conditional path split; Rust surface scan sufficient for future test handoff.

**Stop conditions:** Stop once every scoped key has a verified storage unit, at least one live consumer, and a negative/positive role in stock refinery and stock war-factory flows. Defer only wider mission/timing questions.

## 1. Overview

The four terms are not interchangeable. `QueueingCell` is a cell offset from the building's packed/NW anchor, used by `Mission_Harvest` fallback staging after a failed/too-far return path. `DockingOffset%d` is a 3-int lepton offset vector stored in a `NumberOfDocks`-sized array and added to the building's coordinate by `BuildingClass::GetDockCoord`; for multi-dock buildings the selected index is the `RadioClass` contact slot. `ExitCoord` is a 3-int lepton offset from `BuildingClass` world coordinates and drives stock land war-factory initial unlimbo through `GetExitCoord`.

For stock refineries, the player-visible cells split three ways: accepted `CAN_DOCK` target is hardcoded `NW+(3,1)`; waiting/fallback seed is `QueueingCell=4,1`; normal stock cargo-empty exit installs no new exit destination at all. Conditional reciprocal-link release, sell/damage/temporal interrupts, and stock land war-factory production are separate paths with different anchors.

## 2. Class Layout / Key Offsets

| Owner | Offset / field | Meaning | Evidence | Active in YR |
|---|---:|---|---|---|
| `BuildingTypeClass` | `+0x1618/+0x161C` | `QueueingCell` X/Y as signed/word-sized cell offsets | parser `0x00461506..0x00461526`; consumer `0x0073ED25..0x0073ED3B` | Yes, conditional fallback |
| `BuildingTypeClass` | `+0x1780` | `NumberOfDocks` int | parser `0x00464938..0x00464945`; constructor `0x0043BCBD` | Yes |
| `BuildingTypeClass` | `+0x1788` | `DockingOffset%d` array pointer, entries are 3 ints / 12 bytes | parser `0x0046499D..0x00464A47`; consumer `0x00447B20` | Yes |
| `BuildingTypeClass` | `+0x178C` | docking-offset vector capacity | parser resize branch `0x0046494B..0x0046499D` | Yes |
| `BuildingTypeClass` | `+0xEC8/+0xECC/+0xED0` | `ExitCoord` X/Y/Z lepton offsets | parser `0x00460FB6..0x00460FDF`; consumer `0x0044F640` | Yes |
| `BuildingTypeClass` | `+0x16B3` | `DockUnload=yes` | `0x0043C2D0` case `0x15`; stock refineries | Yes |
| `BuildingTypeClass` | `+0x16BB` | `Refinery=yes` | `0x00447B20`; `0x0073D630` state 3/4 | Yes |
| `BuildingTypeClass` | `+0x16CB` | `Helipad=yes` | `0x00447B20`; airfield reports | Yes for airfields |
| `BuildingTypeClass` | `+0x16A9` | `UnitRepair=yes` | `0x00447B20`; depot pad path | Conditional |
| `BuildingTypeClass` | `+0x16BD` | `WeaponsFactory=yes` | `0x00443C60`, `0x0044EFB0` | Yes for WFs |
| `RadioClass` | `+0xE4/+0xE8` | contacts array pointer/capacity; pad slot source | `0x0065AD90`, `0x0065ADF0`, `0x0065AE60` | Yes |
| `BuildingClass` | `+0x9C/+0xA0/+0xA4` | building world coords used by `ExitCoord` | `0x0044F640` | Yes |
| Unit/building | `+0x2E4` | reciprocal dock-link selector for release helper | `0x0073D63B`, `0x004595C0` | Conditional; not stock zero-link unload |

## 3. Core Logic

### 3.1 INI parsing and units

`BuildingTypeClass::ReadINI @ 0x0045FE50` reads all three reference-point keys:

- `ExitCoord` is read with `CCINIClass__Read3Int` at `0x00460FB6..0x00460FDF` into `+0xEC8/+0xECC/+0xED0`. It preserves all three integer components. Consumer `GetExitCoord @ 0x0044F640` adds those integers directly to building world coords, so the unit is leptons.
- `QueueingCell` is read at `0x00461506..0x00461526` into `+0x1618/+0x161C`; the live fallback consumer adds those values after converting refinery coords to a packed cell, so the unit is cells.
- `NumberOfDocks` is read at `0x00464938..0x00464945` into `+0x1780`.
- If `NumberOfDocks` grows, the docking vector is resized and new entries from the old count to the new count are explicitly zeroed at `0x00464973..0x00464990`.
- Then for each index `< NumberOfDocks`, the reader formats `DockingOffset%d` at `0x004649B7`, reads 3 ints via `CCINIClass__Read3Int`, and writes the 12-byte entry to `+0x1788 + index*12`.

Tiny details:

- `DockingOffset%d` is not bounded by "present art keys"; the binary loops to the current `NumberOfDocks`.
- Missing newly-added offset entries default to `(0,0,0)` after resize.
- `NumberOfDocks` later clamps to at least one contact slot during building construction, but the type field itself is read as an int.
- `ExitCoord` invalid sentinel means "fall back to building center" in `GetExitCoord`, not `(0,0,0)`.

### 3.2 NumberOfDocks becomes RadioClass capacity

`BuildingClass::Constructor @ 0x0043B740` reads `BuildingType+0x1780` at `0x0043BCBD`, clamps values below `1` to `1` at `0x0043BCC3..0x0043BCC8`, and calls `RadioClass::Set_Contact_Count @ 0x0065AE60`.

`Set_Contact_Count` only grows the vector. It calls the vector resize when requested count exceeds current `RadioClass+0xE8`, then fills each newly created slot with `0`. It does not shrink existing contacts.

Active stock examples:

- `GAREFN/NAREFN NumberOfDocks=1` -> one contact slot.
- `GAAIRC/AMRADR NumberOfDocks=4` -> four contact slots and four pad identities.

### 3.3 DockingOffset%d selection and coordinate origin

`BuildingClass::GetDockCoord @ 0x00447B20` is the primary `DockingOffset` consumer.

Branch summary:

1. `Weeder=yes`: returns packed building cell `+(2,1)` as a centered lepton coordinate.
2. `Refinery=yes`: returns the building `+0x48` coords through `FUN_005F6C80`; stock refinery `GetDockCoord` does not read `DockingOffset`.
3. `Bunker=yes` with a sender: chooses one of four half-cell offsets from the building coordinate based on angle to the sender.
4. Non-helipad and non-unit-repair buildings without dock offsets return building coords.
5. `Helipad=yes` or `UnitRepair=yes`:
   - `NumberOfDocks == 0`: return building coords.
   - `NumberOfDocks == 1`: add `DockingOffset0` to building `+0x48` coords.
   - `NumberOfDocks > 1`: call `RadioClass::FindDockSlot(contact)`, then add `DockingOffset[slot]` to building `+0x48` coords. If the slot is invalid, fall back to building coords.

`RadioClass::FindDockSlot @ 0x0065AD90` scans contacts linearly and returns the first index whose pointer equals the sender; missing sender returns `-1`.

Evidence proves the offset is added to the building's `+0x48` coordinate result, not to the packed/NW origin. The exact semantic of `+0x48` for BuildingClass is object-coordinate dependent, but `CellClass +0x48` reports center coords in the sibling cell-reference report; Rust should not treat `DockingOffset%d` as a raw cell offset from `pLocation`.

### 3.4 Stock refinery accepted cell vs waiting seed

`BuildingClass::Receive_Radio @ 0x0043C2D0` case `0x0E` is the receiver-side `CAN_DOCK` branch. For stock `DockUnload=yes` / `Refinery=yes` refineries it uses the building packed/NW cell from vtable `+0x1B8`, adds `(3,1)`, gets that `CellClass`, sends it as message `0x12`, and only sends `0x18`/`0x16` when the unit replies `0x14` (already at the target).

This path does not read `QueueingCell` and does not read `DockingOffset%d`.

Stock refinery example:

- `GAREFN` at `(rx,ry)` accepted `CAN_DOCK` target: `(rx+3, ry+1)`.
- `artmd.ini [GAREFN] QueueingCell=4,1`: not the accepted cell.
- `artmd.ini [GAREFN] RemoveOccupy1=3,1`: matches the visible pad cell / opened foundation cell, but this report treats `RemoveOccupy` as corroborating INI context, not a direct `0x0E` input.

### 3.5 QueueingCell use: Mission_Harvest fallback staging

`UnitClass::Mission_Harvest @ 0x0073E5E0` return state 2 is the verified stock `QueueingCell` consumer.

For stock HARV:

- Close branch computes object-to-object lepton distance to the refinery and compares `distance <= HarvesterTooFarDistance * 256`.
- If close, it sends radio `0x02`; only reply `1` advances state to `3`.
- If the close path does not complete, it runs a fallback dock search and only assigns a movement target when distance to fallback refinery is strictly `> 0x300` leptons.
- That fallback converts the refinery coordinate to a packed/NW cell, reads `BuildingType+0x1618/+0x161C`, adds `QueueingCell`, and calls `Find_Nearby_Passable_Cell` from that seed.

For stock CMIN, the same family is active but the teleporter branch makes fallback staging easier to reach after close HELLO cannot proceed. Prior report `CHRONO_MINER_REFINERY_CONTACT_SATURATION_QUEUE_EVICTION_GHIDRA_REPORT.md` verifies the CMIN/HARV split.

Therefore `QueueingCell=4,1` is a wait/fallback staging seed, not the accepted pad and not normal stock cargo-empty exit.

### 3.6 Stock refinery cargo-empty exit

`UnitClass::Mission_Deploy_Building @ 0x0073D630` splits at entry on unit `+0x2E4`.

For stock zero-link `CMIN/HARV -> GAREFN/NAREFN`:

- `BuildingClass::Receive_Radio(0x15)` queues sender mission `0x10` and does not write reciprocal `+0x2E4`.
- State 3 empty-storage gate writes state `4` and returns.
- State 4 re-finds the refinery at the miner cell plus global `(-1,0)`, waits if refinery slot 8 `+0x57C` is live, clears `unit+0x6D1`, sets/queues Harvest mission `0x0A`, and may send radio `3`.
- No `ExitCoord`, no `QueueingCell`, no `DockingOffset`, no `Find_Nearby_Passable_Cell`, no `Force_Track(0x47)`, and no new exit destination are used by normal stock cargo-empty exit.

Conditional reciprocal-link release:

- If unit `+0x2E4 != 0`, `Mission_Deploy_Building` calls `BuildingClass::ReleaseDockedHarvester @ 0x004595C0`.
- That helper calls locomotor `Force_Track(0x47)` with building `+0x48` coords plus `(-0x80,+0x80)` leptons.
- It computes a destination seed from building packed/NW `(-1,+1)`, runs `Find_Nearby_Passable_Cell`, sets unit destination, then sets mission Move `2`.
- This branch is live but conditional; it is not stock zero-link unload completion.

### 3.7 Stock land war-factory production exit

`BuildingClass::ExitObject_Main @ 0x00443C60` bypasses `GetDockCellForObject` for stock land war factories when `WeaponsFactory=yes` and `Naval=no`. It calls `GetExitCoord @ 0x0044F640`, then unlimbos the produced vehicle at that lepton coord with facing byte `0x40`.

`GetExitCoord` behavior:

- If `ExitCoord` equals the invalid coord sentinel, return building `+0x48` coords.
- Otherwise add `BuildingType+0xEC8/+0xECC/+0xED0` directly to `BuildingClass+0x9C/+0xA0/+0xA4`.

Stock `GAWEAP/NAWEAP/YAWEAP` have `ExitCoord=512,256,0`, so initial unlimbo is building anchor plus `(2,1)` cells in lepton space. `QueueingCell` and `DockingOffset%d` are not inputs to stock land war-factory spawn.

`GetDockCellForObject @ 0x0044EFB0` remains active for other exit/dock helpers and naval weapons factory branches. It uses fixed branch order, optional fallback cell, foundation `ExitList`, and `CanEnterCell` checks; it does not read `QueueingCell` or `DockingOffset%d` for stock land war-factory spawn.

### 3.8 Direction-dependent approach/release reference cells

Verified direction-dependent parts in this slice:

- `BuildingClass::GetDockCoord` bunker branch computes angle from sender to building and chooses one of four half-cell coordinate offsets: NE-ish `(+0x80,-0x80)`, SE-ish `(+0x80,+0x80)`, SW-ish `(-0x80,+0x80)`, NW-ish `(-0x80,-0x80)`.
- `ExitObject_Main` non-WF generic production/exit paths compute facing from building coords to selected dock cell, then use adjacent/foundation-clamped cells. This is active but outside stock refinery/stock land WF coordinate selection.
- Conditional `ReleaseDockedHarvester` uses a fixed west/south release reference: building coords `(-0x80,+0x80)` and packed NW `(-1,+1)` seed. It is not based on `QueueingCell`.

No stock refinery accepted cell in `0x0E` is direction-dependent: it is always NW `(3,1)`.

## 4. INI Keys

| Key | Stock value / examples | Binary effect | Active in YR |
|---|---|---|---|
| `QueueingCell` | `GAREFN/NAREFN=4,1` in `artmd.ini` | cell offset seed for `Mission_Harvest` fallback | Yes, conditional |
| `DockingOffset%d` | `GAAIRC` four offsets; `NADEPT=128,0,0`; `GAREFN/NAREFN` only commented | lepton vector array for `GetDockCoord` on helipad/unit-repair/multi-dock-style paths | Yes where consumer branch active |
| `NumberOfDocks` | refineries `1`, airfields `4`, depots `1` | type field, contact capacity, docking vector loop bound | Yes |
| `ExitCoord` | stock WFs `512,256,0`; barracks often `-64,64,0` or `0,0,0` | lepton offset added to building world coords by `GetExitCoord`; stock land WF spawn source | Yes for WFs |
| `DockUnload` | refineries `yes` | stock refinery `0x0E`/`0x15` branch | Yes |
| `Refinery` | refineries `yes` | refinery `GetDockCoord` branch and unload FSM context | Yes |
| `WeaponsFactory` | stock WFs `yes` | selects stock land WF `GetExitCoord` branch; naval WFs use other dock helper branches | Yes |
| `UnitReload` / `Helipad` | stock airfields | airfield radio/dock pad context | Yes |
| `UnitRepair` | stock depots | service depot pad branch using `DockingOffset0` | Yes |

## 5. Integration Points

| Function | Relationship |
|---|---|
| `BuildingTypeClass::ReadINI @ 0x0045FE50` | parses and stores `QueueingCell`, `DockingOffset%d`, `ExitCoord`, `NumberOfDocks` |
| `BuildingClass::Constructor @ 0x0043B740` | converts `NumberOfDocks` to `RadioClass` contact capacity with min one |
| `RadioClass::FindDockSlot @ 0x0065AD90` | maps contact pointer to pad index for multi-dock coordinates |
| `BuildingClass::GetDockCoord @ 0x00447B20` | consumes `DockingOffset%d` for helipad/unit-repair/multi-dock paths |
| `BuildingClass::Receive_Radio @ 0x0043C2D0` | stock refinery accepted target `(NW+3,NW+1)` and `0x15` handoff |
| `UnitClass::Mission_Harvest @ 0x0073E5E0` | consumes `QueueingCell` for fallback/wait staging |
| `UnitClass::Mission_Deploy_Building @ 0x0073D630` | stock zero-link unload exit and conditional release-helper branch |
| `BuildingClass::ReleaseDockedHarvester @ 0x004595C0` | conditional reciprocal-link exit anchor `(-1,+1)` plus force-track lepton prelude |
| `BuildingClass::ExitObject_Main @ 0x00443C60` | stock land WF production exits through `GetExitCoord` |
| `BuildingClass::GetExitCoord @ 0x0044F640` | consumes `ExitCoord` as lepton offset from building coords |
| `BuildingClass::GetDockCellForObject @ 0x0044EFB0` | non-stock-WF/generic dock-cell helper; negative proof for stock land WFs |

## 6. Current Rust Implementation Status

No Rust was edited. Scanned surfaces:

- `src/rules/object_type.rs` parses `DockPad`, `ExitCoord`, and `NumberOfDocks`; current comments already say `DockingOffset%d` is 256-lepton based and zero-padded to `NumberOfDocks`.
- `src/rules/art_data.rs` parses `QueueingCell` and `DockingOffset0..7`; `ruleset.rs` merges and sizes pads to `NumberOfDocks`.
- `src/sim/docking/pad_geometry.rs::pad_cell_for` treats `DockingOffset` as building-center-relative. Binary evidence says `GetDockCoord` adds offsets to building `+0x48`; whether that equals Rust's geometric center for every building kind should be kept as a verification point, not assumed for all branches.
- `src/sim/miner/miner_dock_sequence.rs` has a split between `refinery_queue_cell` (`QueueingCell=4,1`) and `refinery_can_dock_queue_cell` (`NW+3,NW+1`), matching the binary distinction.
- `src/sim/miner/miner_dock_sequence.rs` now documents normal stock exit as no explicit exit move; helper `refinery_exit_cell` remains test-only/conditional.
- `src/sim/production/production_spawn.rs::find_exact_exitcoord_spawn_cell` uses exact `ExitCoord`; `preferred_exit_offsets` still has adjacent fallback candidates for generic paths.
- `src/sim/docking/aircraft_dock.rs` has `AirfieldDocks` slots/queues; prior airfield report says slot identity should be contact-slot-backed, while FIFO auto-promotion is not a verified RadioClass primitive.

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| `ExitCoord` parser/storage | verified | `0x00460FB6..0x00460FDF` | none |
| `QueueingCell` parser/storage | verified | `0x00461506..0x00461526` | none |
| `NumberOfDocks` parser/storage | verified | `0x00464938..0x00464945` | none |
| `DockingOffset%d` parser/storage | verified | `0x0046499D..0x00464A47` | none |
| `NumberOfDocks` -> contact capacity | verified | `0x0043BCBD..0x0043BCD0`, `0x0065AE60` | none |
| Contact slot -> docking offset index | verified | `0x00447B20`, `0x0065AD90` | none |
| Stock refinery accepted cell | verified | `0x0043C2D0` case `0x0E`; prior receiver reports | none |
| Stock refinery QueueingCell fallback | verified | `0x0073ED25..0x0073ED3B`; HARV state-2 report | exact rendered two-miner timing deferred |
| Stock refinery normal exit no destination | verified | `0x0073D630` state 4; HARV/CMIN stock exit reports | same-frame promotion timing deferred |
| Conditional `ReleaseDockedHarvester` exit anchor | verified | `0x004595C0`; caller `0x0073D66D` | frequency in mods/interrupts outside scope |
| Stock land war-factory `ExitCoord` spawn | verified | `0x00443C60`, `0x0044F640`; stock INI | queue restart/refund after blocked unlimbo outside scope |
| `GetDockCellForObject` negative for stock WFs | verified | `0x0044EFB0`, `0x00443C60` | naval yard behavior out-of-scope |
| Bunker direction-dependent dock coord | verified | `0x00447B20` | not stock refinery/WF |
| Rust surface scan | verified structurally | `src/rules`, `src/sim/miner`, `src/sim/docking`, `src/sim/production` | tests not run in this research-only slot |

## 8. Open Questions - Final State of the Investigation Log

- `[RESOLVED] OQ-01 - What mode applies? -> Exhaustive-slice over reference-point keys and stock refinery/WF consumers; wider mission timing is non-scope.` (evidence: user scope; function list above)
- `[RESOLVED] OQ-02 - Does output report already exist? -> No.` (evidence: `Test-Path` before write)
- `[RESOLVED] OQ-03 - Where is `ExitCoord` read and what are its units? -> Read as 3 ints into `+0xEC8/+0xECC/+0xED0`; consumed as lepton offsets by `GetExitCoord`.` (evidence: `0x00460FB6..0x00460FDF`, `0x0044F640`)
- `[RESOLVED] OQ-04 - Where is `QueueingCell` read and what are its units? -> Read into `+0x1618/+0x161C`; consumed as cell offsets by `Mission_Harvest` fallback.` (evidence: `0x00461506..0x00461526`, `0x0073ED25..0x0073ED3B`)
- `[RESOLVED] OQ-05 - Where is `NumberOfDocks` read and what does it size? -> Read to `+0x1780`, then sizes contact capacity and docking-offset loop.` (evidence: `0x00464938..0x00464945`, `0x0043BCBD`, `0x0046499D`)
- `[RESOLVED] OQ-06 - Where is `DockingOffset%d` read and what are its units? -> Read as 3-int entries into a 12-byte array; `GetDockCoord` adds entries to building coords, so they are leptons.` (evidence: `0x004649B7`, `0x00447B20`)
- `[RESOLVED] OQ-07 - Does stock refinery accepted docking use `QueueingCell`? -> No; `0x0E` hardcodes `NW+(3,1)`.` (evidence: `0x0043C2D0`)
- `[RESOLVED] OQ-08 - Does stock refinery accepted docking use `DockingOffset0`? -> No; stock `GetDockCoord` refinery branch returns building coords and `0x0E` does not call pad offsets.` (evidence: `0x00447B20`, `0x0043C2D0`)
- `[RESOLVED] OQ-09 - Does stock return fallback use `QueueingCell`? -> Yes, after failed/too-far state-2 path, with `QueueingCell=4,1` seed for stock refineries.` (evidence: `0x0073E5E0`, `artmd.ini`)
- `[RESOLVED] OQ-10 - Does normal stock cargo-empty exit install an exit cell? -> No; zero-link state 4 sets Harvest/contacts, no destination or force-track.` (evidence: `0x0073D630`)
- `[RESOLVED] OQ-11 - Where does conditional release exit move? -> `ReleaseDockedHarvester` uses building coords `(-0x80,+0x80)` for force-track and packed NW `(-1,+1)` as passable destination seed.` (evidence: `0x004595C0`)
- `[RESOLVED] OQ-12 - What drives stock land war-factory spawn? -> `ExitCoord=512,256,0` through `GetExitCoord`, not `GetDockCellForObject`.` (evidence: `0x00443C60`, `0x0044F640`, `rulesmd.ini`)
- `[RESOLVED] OQ-13 - Does `GetDockCellForObject` use `QueueingCell`/`DockingOffset` for stock WFs? -> No; stock land WF bypasses it and the helper itself has no such reads.` (evidence: `0x0044EFB0`, `0x00443C60`)
- `[RESOLVED] OQ-14 - What is pad index identity for multi-dock buildings? -> The radio contact slot index via `FindDockSlot`.` (evidence: `0x00447B20`, `0x0065AD90`)
- `[RESOLVED] OQ-15 - What happens if no contact slot matches in multi-dock `GetDockCoord`? -> It falls back to building coords.` (evidence: `0x00447B20`)
- `[RESOLVED] OQ-16 - Does `NumberOfDocks=0` create zero contact slots? -> No; constructor clamps contact capacity to at least one.` (evidence: `0x0043BCC3..0x0043BCC8`)
- `[RESOLVED] OQ-17 - Are stock WFs active YR for `ExitCoord`? -> Yes, `GAWEAP/NAWEAP/YAWEAP` have `WeaponsFactory=yes`, `Factory=UnitType`, `ExitCoord=512,256,0`, no `Naval=yes`.` (evidence: `rulesmd.ini`; `0x00443C60`)
- `[RESOLVED] OQ-18 - Are stock refineries active YR for this split? -> Yes, `GAREFN/NAREFN` have `DockUnload=yes`, `Refinery=yes`, `NumberOfDocks=1`, `QueueingCell=4,1`.` (evidence: `rulesmd.ini`, `artmd.ini`)
- `[RESOLVED] OQ-19 - What Rust surfaces reflect the split? -> miner helpers split queue/can-dock cells; production spawn has exact `ExitCoord`; pad geometry handles docking pads.` (evidence: code scan)
- `[DEFERRED] OQ-20 - Exact same-rendered-frame promotion when a second miner waits through state-4 radio cleanup.` (category: `needs-runtime-debugger`; reason: static evidence proves contacts/cells, not object iteration frame order; next-step-if-pursued: runtime watch contacts and two miners across the handoff)
- `[DEFERRED] OQ-21 - Naval weapons factory/Yard exit coordinate contract.` (category: `out-of-scope`; reason: user asked refinery/war-factory anchors and general docking keys; naval branch is a separate coordinate system; next-step-if-pursued: trace `Naval && WeaponsFactory` branch and yard INI)
- `[DEFERRED] OQ-22 - Whether BuildingClass `+0x48` equals Rust geometric-center math for every `DockingOffset` building.` (category: `requires-different-system-context`; reason: `GetDockCoord` addition point is verified, but full BuildingClass coordinate virtual is covered by building-anchor slot; next-step-if-pursued: reconcile with slot-5 foundation-anchor report)

## 9. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| `QueueingCell` is a cell-offset fallback seed, not the accepted refinery dock cell | `0x0073ED25..0x0073ED3B`; `0x0043C2D0` negative | mostly matched | `src/sim/miner/miner_dock_sequence.rs::refinery_queue_cell`, `refinery_can_dock_queue_cell`; `src/sim/miner/miner_system.rs` | Preserve split: wait/far staging can use `(4,1)`, accepted `0x0E` target uses `(3,1)` | Two miners at GAREFN: second stages at/near `(rx+4,ry+1)` but does not enter unload until it reaches accepted `(rx+3,ry+1)` and receives final burst | Do not rename `QueueingCell` to "dock pad" or use it for `0x0E` |
| Normal stock cargo-empty refinery exit installs no new exit destination | `0x0073D630` state 4; no `+0x480` / no `Force_Track` in zero-link branch | current dirty Rust appears aligned | `src/sim/miner/miner_dock_sequence.rs::phase_departing`; miner tests | Hand directly to Harvest/Search after contact cleanup; keep `refinery_exit_cell` conditional/test-only | Full HARV/CMIN unload ends without moving to `(rx+4,ry+1)` or `(-1,+1)` release cell | Do not reuse `ReleaseDockedHarvester` for stock zero-link completion |
| Conditional reciprocal-link release uses `(-0x80,+0x80)` lepton force-track and packed `NW+(-1,+1)` passable seed | `0x004595C0`; caller only nonzero `+0x2E4` branch at `0x0073D66D` | partly represented by legacy helpers; scope should stay conditional | interrupt/sell/destroy docked-miner code; conditional release tests | Keep separate from stock exit; use when reciprocal link/interrupt path is actually reached | Sell/destroy/refinery interrupt ejects docked miner with force-track `0x47` and west/south release seed | Do not make this the generic refinery unload exit |
| `DockingOffset%d` is a lepton vector added to `BuildingClass +0x48` coords, selected by radio contact slot for multi-dock buildings | `0x004649B7`, `0x00447B20`, `0x0065AD90` | Rust center-relative math may match stock airfields but should be treated as BuildingCoord-relative contract | `src/sim/docking/pad_geometry.rs`, `src/sim/docking/aircraft_dock.rs`, depot docking | Pad index must remain tied to contact slot; offset conversion should be verified against building coord helper | Four aircraft reserve/contact slots 0..3 and land on GAAIRC pads matching `DockingOffset0..3` | Do not allocate a Rust FIFO pad index that diverges from radio contact slot |
| `NumberOfDocks` sizes both contact capacity and `DockingOffset` loop, with new entries zeroed and runtime capacity clamped to at least 1 | `0x00464938..0x00464990`, `0x0043BCBD..0x0043BCD0`, `0x0065AE60` | Rust pads zero-fill to `NumberOfDocks`; airfield FIFO auto-promotion remains not binary-backed | `src/rules/ruleset.rs`, `src/sim/docking/aircraft_dock.rs`, `src/sim/miner/miner_dock.rs` | Preserve capacity and zero-fill; avoid treating `NumberOfDocks=0` as no radio contacts | Mod building `NumberOfDocks=4` with two offsets has two explicit pads plus two zero-offset pads, contact capacity 4 | Do not shrink/ignore contacts because art omitted later `DockingOffsetN` |
| Stock land WFs use `ExitCoord=512,256,0` as initial unlimbo coord; no `QueueingCell`/`DockingOffset` input | `0x00443C60`, `0x0044F640`, `0x0044EFB0` negative | exact helper exists; generic fallback candidates remain a parity risk if used for stock WF blocked exit | `src/sim/production/production_spawn.rs::find_exact_exitcoord_spawn_cell`, `preferred_exit_offsets` | For stock land WFs, primary spawn is exactly `(rx+2,ry+1)`; blocked primary should not silently nearest-cell fallback in this path | Block `(rx+2,ry+1)` for GAWEAP and verify initial spawn fails/defers instead of appearing adjacent | Do not use `GetDockCellForObject`, `ExitList`, `QueueingCell`, or `DockingOffset0` for stock land WF initial spawn |

Proposed Rust test names:

- `refinery_can_dock_target_uses_removeoccupy_cell_not_queueing_cell`
- `harvester_return_fallback_seeds_from_queueing_cell_only_after_failed_close_path`
- `stock_refinery_unload_exit_does_not_install_queue_or_release_destination`
- `reciprocal_release_uses_west_south_anchor_not_art_queueing_cell`
- `airfield_pad_index_matches_radio_contact_slot`
- `number_of_docks_zero_fills_missing_docking_offsets`
- `stock_war_factory_exitcoord_blocked_does_not_probe_neighbor_cells`

### Stale Docs / Follow-up Docs

- `docs/research/miner/traces/CHRONO_MINER_POST_DUMP_EXIT_MOVEMENT_SWARM_20260520_TRACE.md`: replace any unsuperseded statement saying normal stock post-dump exit uses `ReleaseDockedHarvester` / `Force_Track(0x47)` / queue-cell destination with: "Normal stock zero-link `CMIN/HARV -> GAREFN/NAREFN` cargo-empty exit does not call `ReleaseDockedHarvester`, does not run `Force_Track(0x47)`, and does not install a `QueueingCell` or `(-1,+1)` exit destination; those belong to conditional reciprocal-link or interrupt paths."
- `docs/research/miner/MISSION_ENTER_REFINERY_DOCK_GHIDRA_REPORT.md`: if it calls `QueueingCell=4,1` the accepted dock cell, replace with: "The accepted receiver `0x0E` target is `building NW+(3,1)`; `QueueingCell=4,1` is a `Mission_Harvest` fallback/wait seed."
- `docs/research/BUILDING_GETDOCKCELLFOROBJECT_STOCK_WAR_FACTORY_EXIT_GHIDRA_REPORT.md`: no replacement needed; this report corroborates its stock land-WF `ExitCoord` finding.

## Negative Facts / Do Not Do

- Do not treat `QueueingCell` as a lepton offset. It is consumed as a cell offset after anchor-cell conversion.
- Do not use `QueueingCell=4,1` for the stock refinery `CAN_DOCK(0x0E)` accepted target; use hardcoded `NW+(3,1)`.
- Do not model normal stock refinery cargo-empty exit as `ReleaseDockedHarvester`, `UndockUnit`, `Force_Track(0x47)`, or a move to `QueueingCell`.
- Do not treat `DockingOffset%d` as a cell offset or as stock refinery accepted-pad data.
- Do not let multi-pad Rust reservation slot diverge from the radio/contact slot if aiming for gamemd pad identity.
- Do not treat `NumberOfDocks=0` as zero contact capacity; constructor clamps to one.
- Do not use `QueueingCell`, `DockingOffset%d`, or `GetDockCellForObject` for stock land war-factory initial spawn.

## Remaining Uncertainty

- Same-frame rendered ordering for two miners when one stock miner clears state-4 contacts and a waiting miner resumes requires runtime debugging.
- Naval factory/shipyard exit/docking reference points were intentionally not covered.
- Full proof that Rust's geometric-center `pad_cell_for` equals every relevant BuildingClass `+0x48` coordinate branch belongs with the building-anchor/foundation slot.

## Sources

- Ghidra live decompile/assembly: `BuildingTypeClass::ReadINI @ 0x0045FE50`
- Ghidra live decompile: `BuildingClass::Constructor @ 0x0043B740`
- Ghidra live decompile: `BuildingClass::GetDockCoord @ 0x00447B20`
- Ghidra live decompile: `BuildingClass::Receive_Radio @ 0x0043C2D0`
- Ghidra live decompile: `UnitClass::Mission_Harvest @ 0x0073E5E0`
- Ghidra live decompile: `UnitClass::Mission_Deploy_Building @ 0x0073D630`
- Ghidra live decompile: `BuildingClass::ReleaseDockedHarvester @ 0x004595C0`
- Ghidra live decompile: `BuildingClass::ExitObject_Main @ 0x00443C60`
- Ghidra live decompile: `BuildingClass::GetExitCoord @ 0x0044F640`
- Ghidra live decompile: `BuildingClass::GetDockCellForObject @ 0x0044EFB0`
- Ghidra live decompile: `RadioClass::FindDockSlot @ 0x0065AD90`, `FUN_0065ADF0 @ 0x0065ADF0`, `RadioClass::Set_Contact_Count @ 0x0065AE60`
- `ini/rulesmd.ini`
- `ini/artmd.ini`
- `docs/research/miner/HARV_STATE2_TOOFAR_BRANCH_GHIDRA_REPORT.md`
- `docs/research/miner/HARV_POST_UNLOAD_EXIT_PATH_GHIDRA_REPORT.md`
- `docs/research/CHRONO_MINER_REFINERY_CONTACT_SATURATION_QUEUE_EVICTION_GHIDRA_REPORT.md`
- `docs/research/AIRFIELD_RADIO_CACHEDDOCK_CONTACT_LIFETIME_GHIDRA_REPORT.md`
- `docs/research/BUILDING_RECEIVE_RADIO_REFINERY_0X0E_NON_ACCEPTED_PATHS_GHIDRA_REPORT.md`
- `docs/research/BUILDING_RECEIVE_RADIO_DOCK_CLEARANCE_HANDOFF_EXIT_GHIDRA_REPORT.md`
- `docs/research/BUILDING_GETDOCKCELLFOROBJECT_STOCK_WAR_FACTORY_EXIT_GHIDRA_REPORT.md`
- Rust scan: `src/rules/object_type.rs`, `src/rules/art_data.rs`, `src/rules/ruleset.rs`, `src/sim/miner/miner_dock_sequence.rs`, `src/sim/docking/pad_geometry.rs`, `src/sim/docking/aircraft_dock.rs`, `src/sim/production/production_spawn.rs`
