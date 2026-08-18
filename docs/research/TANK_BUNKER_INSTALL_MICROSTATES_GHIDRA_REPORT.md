# Tank Bunker Install Micro-States + Shared Force-Track Semantics — Ghidra Research Report

**Address(es):** `0x00458E50` (install state machine, primary); callees `0x004c9220` RateTimer/FacingClass set, `0x004c9480` FacingClass is-rotating, `0x004c93d0` FacingClass current, `0x0047c520` Look_up_building_in_cell, `0x0047c3d0` CellClass::Find_Nearest_Object; exit helpers `0x004595C0`/`0x004593A0` (shared force-track).
**Investigation Mode:** exhaustive-slice
**Claimed Scope:** the 6-state install micro-states of `0x00458E50` (turn/facing timers, entry cell, force-track choice, blocker-shove), the shared exit force-track semantics, the unit-hide vtable identities, and the bunker admission handshake order. Closes the "Open RE gaps" of `docs/plans/2026-06-02-tank-bunker-lifecycle-design.md`.
**Non-Scope:** the already-verified state-5 install writes / state-4 entry-anim selection / exit-helper teardown sequences (covered by TANK_BUNKER_ENTRY_EXIT / BUNKER_0X2E4_LIFECYCLE_EXIT_CLEAR_PATH / BUNKER_SERVICEDEPOT_0X2E4 reports), bunker combat math, civilian garrison.
**Confidence:** High for the install micro-state structure, the facing-timer identity, the force-track octant map, the blocker-shove, and the handshake order. Medium for the `+0x150`/`+0x480` vtable *names* (behavior verified from call shape + exit-helper pairing; the COL-verified decompile is deferred — this DB's UnitClass RTTI backlink is not cleanly analyzed). Low/Deferred for the `+0x214` reader.
**Active in YR:** Conditional — runs only for `BuildingType+0x16AB` (`Bunker=yes`) buildings on the bunker mission; stock `[NATBNK]` only.

## 1. Overview

The bunker install machine (`0x00458E50`, sole caller `MissionRepairAndProduce @ 0x0044B780`, `Bunker=yes`-gated) is a **facing-driven**, not timer-driven, 6-state machine on the building, operating on the *candidate unit*. The single most important correction to the prior design assumption: the "RateTimer/CDTimer" calls are **the unit's body `FacingClass` at `unit+0x388`** — there are **no magic frame-count wait durations**. Each "wait" is "has the unit finished rotating to the desired facing," whose length is `|Δfacing| / ROT` (the unit's turn rate). The constant `0x8000` is a **desired facing value (South)**, not a duration.

The install choreographs: arrive on the footprint → shove foundation blockers → turn to face the building → diagonal sub-cell track-step onto the building coord → turn to South → entry anim → hide+install. The exit force-track (`0x47`) is the already-researched chrono-miner curved sub-cell departure.

## 2. Class Layout / Key Offsets

| Object | Offset | Meaning | Evidence | Active in YR |
|---|---:|---|---|---|
| BuildingClass | `+0x2E4` | installed-unit ptr (candidate during states; written state 5) | `0x00458e5b` read, `0x00459301` write | Conditional |
| BuildingClass | `+0x718` | install state 0–6 | switch `0x00458e99`; writes per state | Conditional |
| BuildingClass | `+0x520` | TypeClass ptr (entry-anim slot base) | `0x0045926b` `[ESI+0x520]+0x11f4` | Yes |
| BuildingClass | `+0x9c/+0xa0/+0xa4` | Location x/y/z (up-sound position) | `0x0045934a` `ADD ESI,0x9c` | Yes |
| **unit** | **`+0x388`** | **body `FacingClass` (target/current/start-frame/duration/ROT)** | `LEA ECX,[EBP+0x388]` before every `0x004c9220/0x004c9480/0x004c93d0` call (`0x004590e4`, `0x00459101`, `0x0045911a`, `0x0045921a`, `0x0045923f`) | Yes |
| unit | `+0x214` (`[0x85]`) | cleared to `-1` at install (sentinel) | `0x00459315` `MOV [EBP+0x214],EDI(-1)` | Conditional |
| unit | `+0x674` (`[0x19d]`) | active `ILocomotion*` | `0x00458ec9` null-assert, `0x004591af` `+0x70` call | Yes |
| FacingClass | `+0x0` (short) | current/target facing (16-bit) | `0x004c9220`/`0x004c93d0` body | Yes |
| FacingClass | `+0x8` (int) | start frame snapshot (`g_CurrentFrameCounter`), `-1` = idle | `0x004c9220` writes `param_1+4` (=byte 8) | Yes |
| FacingClass | `+0x10` (int) | turn duration = `\|Δfacing\| / ROT` frames | `0x004c9220` `... / sVar1` | Yes |
| FacingClass | `+0x14` (short) | ROT (turn rate); `<=0` ⇒ instant | `0x004c9480` `*(short*)(p+0x14)` gate | Yes |

## 3. Core Logic — the 6-state install machine (`0x00458E50`)

`ESI` = building, `EBP` = candidate unit. **Preflight (every tick, `0x00458e5b`):** `unit = building+0x2E4`; if null → `FootClass__GetDestination(0)` (`0x0065ad30`); if still null OR `unit.WhatAmI()` (`vtable+0x2c`) `!= 1` (not a UnitClass) → `building+0x718 = 0`, `building.Queue_Mission(Guard=5, 0)` (`vtable+0x1e8`), return. Then `switch(building+0x718)` 0–5 (jump table `0x0045937c`); `>5` (state 6 = occupied) → return (no-op).

| State | Address | Behavior (verified) | Exit |
|---|---|---|---|
| **0 — arrive + shove** | `0x00458eaf` | `unit.vtable+0x1bc()` → unit's current CellClass; `Look_up_building_in_cell` (`0x0047c520`) → must `== building` else return. If locomotor (`unit+0x674`) `vtable+0x10()` returns nonzero (still moving) → return (wait). Else, iterate the building **foundation offset list** (`building.vtable+0x108`, base cell from `vtable+0x1b8`, sentinel `(0x7fff,0x7fff)`): per cell `MapClass__Get_CellClass`, `CellClass__Find_Nearest_Object(range=(0x80,0x80), 0, building)` (`0x0047c3d0`); if found `obj != unit` → `building.vtable+0x48()` building coords, then **`obj.vtable+0x174(coords)`** (scatter/move-away the blocker). | → state 1 |
| **1 — wait clear + face building** | `0x00458fa9` | Re-scan the foundation. An object counts as a hard blocker iff `(obj+0x14 & 4) && (obj+0x5a4 == 0)` (`0x0045902f`–`0x00459044`) → stay in state 1. A "found-flag" (`[ESP+0x13]=1`, `0x00459032`) is set whenever **any** non-unit object sits on the footprint; if the scan completes with the flag set → return (keep waiting). When the footprint is empty: facing = `atan2(building.y − unit.y, unit.x − building.x)` (`Math__atan2 0x004cae30`) → `ftol` (`0x007c5f00`) → `+ 0x7fff` (RA2 facing-convention offset, `0x004590cd`); `FacingClass__Set(unit+0x388, facing)` (`0x004c9220`). | → state 2 |
| **2 — wait turn + diagonal track-step** | `0x00459101` | `FacingClass__IsRotating(unit+0x388)` (`0x004c9480`); if still turning → return. Else read `FacingClass__Current(unit+0x388)` (`0x004c93d0`); **octant = `(facing >> 7) + 1 & 0x1FE`**, mapped: `0x40`(NE)→track **`0x43`**, `0xC0`(SE)→**`0x44`**, `0x1C0`(NW)→**`0x46`**, else(SW)→**`0x45`** (`0x0045912d`–`0x0045915c`). Then `building.vtable+0x48()` building coords; locomotor `vtable+0x70(coord = building coords, track)` (force-track — **no `±0x80` offset here**, unlike the exit); `unit.vtable+0x544(0, 0x3FF00000)` (speed = 1.0). | → state 3 |
| **3 — wait arrival + face South** | `0x004591d6` | `unit.vtable+0x1bc()` cell; `Look_up_building_in_cell == building` else return; locomotor `vtable+0x10()` nonzero → return (wait until track-step done). Else `FacingClass__Set(unit+0x388, 0x8000)` — **`0x8000` = 16-bit facing South (`0x80` byte)**. | → state 4 |
| **4 — wait turn + entry anims** | `0x0045923f` | `FacingClass__IsRotating(unit+0x388)`; if turning → return. Else health-gate on `GetHealthRatio(building)` vs `RulesClass+0x1700` (ConditionRed): ratio > ConditionRed → `Type+0x11F4` (SpecialAnim) + `Type+0x1238` (SpecialAnimTwo), damaged-flag `0`; else `Type+0x1204`/`Type+0x1248` (…Damaged), flag `1`. Each non-empty slot → `CreateAnimForSlot(building, slot=0xa then 0xb, flag, …)` (`0x00451890`). | → state 5 |
| **5 — install** | `0x00459301` | (already verified — unchanged) `building+0x2E4=unit`; `unit+0x2E4=building`; `unit+0x214=-1`; `unit.vtable+0x150()` hide; `building+0x718=6`; `unit.Queue_Mission(Guard=5, 1)`; if `RulesClass+0x240 (BunkerWallsUpSound) != -1` → `VocClass__PlayAt` at building Location. | → state 6 (occupied) |

### 3.1 FacingClass timer identity (the OQ1 correction)

`0x004c9220` / `0x004c9480` / `0x004c93d0` all operate on the same 0x16-byte struct (`+0x0` facing short, `+0x8` start-frame int, `+0x10` duration int, `+0x14` ROT short). `__Set(new)` snapshots `g_CurrentFrameCounter` into `+0x8` and computes `+0x10 = |new − prev| / ROT`. `__IsRotating` returns `(CurrentFrame − start) < duration`. `__Current` linearly interpolates between previous and target by elapsed/duration. **Conclusion:** the install's two "waits" are turn-completion checks; their length is data-driven by the unit's ROT and the angular distance, **not** any literal frame constant. `0x8000` is the state-3 desired facing (South), not a wait length.

### 3.2 Shared exit force-track `0x47` (OQ3, already researched)

The exit helpers `ReleaseDockedHarvester @ 0x004595C0` (normal) and `UndockUnit @ 0x004593A0` (sell/destroy) call locomotor `vtable+0x70(track=0x47, x−0x80, y+0x80, z)`. Per `CHRONO_MINER_FORCE_TRACK_0X47_REFINERY_EXIT_GHIDRA_REPORT.md` (verified): `0x47` = TurnTrack[71] → RawTrack[15], a curved 16-point **sub-cell departure** shape (starts `(128,−128,facing 0x80)`, ends `(16,−4,0xBC)`), flags `0` (raw facings used unchanged), no cell-cross marker; `−0x80/+0x80` are **hardcoded leptons** (half-cell), not INI-driven; facing is updated by `Process_Drive_Track`. `src/sim/movement/drive_track.rs` already carries TurnTrack[71]/RawTrack[15]/Track15 points. The **install** tracks `0x43–0x46` are the four diagonal sub-cell entry curves (the install analogue), chosen by octant; their point tables are siblings of track 15 in the same `g_DriveTrackIndex_Table` (read at runtime via the same `Process_Drive_Track`).

## 4. INI Keys

No new keys beyond the verified set: `[NATBNK] Bunker=yes` (`+0x16AB`), `Foundation=2x2`, `NumberOfDocks=1`, `DockingOffset0=-1,-1,0` (commented "unused" — **confirmed unused**: `0x00458E50` never reads a docking offset; the unit reaches the footprint via its own move and `Look_up_building_in_cell` only checks `WhatAmI()==6`). `[AudioVisual] BunkerWallsUpSound/DownSound` (`RulesClass+0x240/+0x244`). `[General] ConditionRed` (`RulesClass+0x1700`).

## 5. Integration Points

Caller: `MissionRepairAndProduce @ 0x0044B780` (sole). Handshake (OQ7): `RADIO_INFERRED_CODES_AND_DOCK_HANDSHAKE_ORDER` §9.4 confirms bunker entry uses **`HELLO(0x02)` then `CAN_ENTER(0x0F)`** for admission; the lifecycle report confirms case `0x15` → `building.Queue_Mission(0x14)` → `MissionRepairAndProduce` → this machine. The install reads the unit body facing (`unit+0x388`) and drives the active locomotor (`unit+0x674` `vtable+0x70`). Tick: building mission path (Rust docks phase). The unit's own approach move is independent (states 0/3 are passive "is it here yet").

## 6. Current Rust Implementation Status

No bunker lifecycle exists (data-only — see the design doc). Relevant reusable primitives confirmed present: `drive_track.rs` has TurnTrack[71]/RawTrack[15]; `FacingClass` exists (used for `barrel_facing`); `conceal`/`reveal`/`remove`/`add_entity_occupancy` exist; `Find_Nearby_Passable_Cell`-style placement exists in `passenger.rs`. The install diagonal tracks `0x43–0x46` are **not** confirmed present in `drive_track.rs` (only `0x47`/track 15 was cited) — verify at implementation.

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| Install state machine structure (states 0–5) | verified | `disassemble 0x00458E50` (full), `decompile 0x00458E50` | none |
| FacingClass timer identity (`unit+0x388`) | verified | `decompile 0x004c9220/0x004c9480/0x004c93d0` + `LEA [EBP+0x388]` sites | none |
| State-2 octant→track map (0x43–0x46) | verified | asm `0x0045911a`–`0x0045915c` | install track point tables not byte-dumped (siblings of track 15) |
| State-1 facing = atan2-to-building | verified | asm `0x0045906c`–`0x004590ea` | none |
| State-3 target facing = South (0x8000) | verified | asm `0x00459221` | none |
| Entry cell = footprint via own move; no dock offset | verified | `decompile 0x0047c520` (WhatAmI==6 only); DockingOffset unused | none |
| Blocker-shove (state 0 scatter / state 1 wait-clear) | verified | asm `0x00458f15`–`0x00459049`; predicate `obj+0x14&4 && obj+0x5a4==0` | `obj.vtable+0x174` exact name (behaviorally = Scatter) |
| Exit force-track 0x47 | verified (prior doc) | `CHRONO_MINER_FORCE_TRACK_0X47_REFINERY_EXIT` | none |
| `+0x150` hide / `+0x480` place identity | touched-not-exhausted | call shape + exit-helper pairing + UndockUnit-no-replace | COL-verified decompile (RTTI backlink not analyzed) |
| `unit+0x214 = -1` reader | deferred | write at `0x00459315`; row-helper doc | reader not traced |
| Handshake order HELLO→CAN_ENTER | verified (prior doc) | `RADIO_INFERRED_CODES…` §9.4 | unit-side *sender function* unverified |

## 8. Open Questions — Final State

- `[RESOLVED] OQ1 — install inter-state wait durations / 0x8000` → the "timers" are the unit body `FacingClass @ unit+0x388`; waits are turn-completion (`|Δfacing|/ROT`, no magic frame counts); `0x8000` is the state-3 desired facing **South**, not a duration. (evidence: `decompile 0x004c9220/0x004c9480/0x004c93d0`; asm `LEA [EBP+0x388]` at `0x004590e4`/`0x00459101`/`0x0045921a`; `0x00459221 MOV word [ESP+0x30],0x8000`)
- `[RESOLVED] OQ2 — entry cell on 2×2` → no dock cell / `DockingOffset` unused; the unit drives onto the footprint via its own move; `Look_up_building_in_cell` returns the first `WhatAmI()==6` building in the unit's current cell and the machine only proceeds if it `== this building`. (evidence: `decompile 0x0047c520`; asm `0x00458eb4`/`0x004591db`; `artmd.ini` DockingOffset0 comment + no read in `0x00458E50`)
- `[RESOLVED] OQ3 — force-track tracks` → install uses diagonal entry tracks `0x43`(NE)/`0x44`(SE)/`0x45`(SW)/`0x46`(NW) by facing octant, target = building coords, **no `±0x80` offset**; exit uses `0x47` = TurnTrack[71]/RawTrack[15] curved sub-cell departure with hardcoded `−0x80/+0x80` lepton offset (prior chrono doc). (evidence: asm `0x0045912d`–`0x004591af`; `CHRONO_MINER_FORCE_TRACK_0X47_REFINERY_EXIT`)
- `[DEFERRED] OQ4 — unit+0x214 reader` (category: bounded-cost-too-high; reason: write to the `-1` sentinel at install verified, but tracing every reader needs a struct-wide field scan; next-step: define the unit struct + `get_field_access_context` on `+0x214`. Behavioral note: it is set to the `-1` "none" sentinel immediately before hiding — model as clearing the unit's pending-nav/target equivalent on install.)
- `[RESOLVED-behavioral / DEFERRED-COL] OQ5 — vtable identities` → `+0x1bc` returns the unit's current CellClass (feeds `Look_up_building_in_cell`, which reads `cell+0xE4`); `+0x544(0, 0x3FF00000)` sets speed = double 1.0; `+0x150` = no-arg hide on install; `+0x480(cell, dir)` = place/Unlimbo on exit. **Light-vs-full-limbo:** `UndockUnit` (sell/destroy) clears the link and force-tracks the unit **without** any `+0x480` re-place, and the bunkered tank fires (combat surface) → `+0x150` is a **light hide that keeps the unit's coordinate and live-object status**, not a full logic-vector limbo. (evidence: asm `0x00458eb4`/`0x004591bе`/`0x0045931b`; exit `decompile 0x004595C0` `+0x480(cell,1)` / `0x004593A0` no `+0x480`.) COL-verified decompile DEFERRED (category: bounded-cost-too-high; this DB's UnitClass RTTI COL→vtable backlink is not analyzed — `get_xrefs_to` on COL base `0x0080cbec` and byte-pattern for the pointer both returned nothing; next-step: locate the UnitClass vtable via a known UnitClass virtual override and `read_memory vtable+0x150`/`+0x480` then decompile).
- `[RESOLVED] OQ6 — blocker-shove` → state 0 scatters every non-unit object on the footprint via `obj.vtable+0x174(building-coords)` after `Find_Nearest_Object(range 0x80)`; state 1 waits until the footprint is empty (hard-blocker predicate `obj+0x14 & 4 && obj+0x5a4 == 0`). (evidence: asm `0x00458f15`–`0x00459049`)
- `[RESOLVED] OQ7 — handshake order` → `HELLO(0x02)` precedes `CAN_ENTER(0x0F)`; install is triggered by `0x15` → building mission `0x14`. (evidence: `RADIO_INFERRED_CODES…` §9.4; lifecycle report case `0x15`.) Unit-side *sender function* of `0x0F`/`0x15` UNVERIFIED — `[DEFERRED]` (category: out-of-scope; not needed for the command-driven Rust admission).

## 9. Implementation Handoff (keyed to the design's `BunkerRuntime` 6-state machine)

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required effect | Acceptance | Risk / do-not-do |
|---|---|---|---|---|---|---|
| Install is facing-driven, no fixed wait frames | `0x00458E50` + `unit+0x388` FacingClass | missing | `BunkerRuntime` states ClearWait/Turn/TurnWait | Drive state transitions off the unit's **turn-completion** (reuse the existing `FacingClass`/ROT), not `MissionTimer` countdowns | unit with low ROT takes proportionally longer to install; high ROT installs faster | **Do NOT add magic frame constants** for the install waits — there are none |
| State-1 facing = atan2 toward building; state-3 facing = South (0x8000) | asm `0x0045906c`/`0x00459221` | missing | install machine | turn unit to face the building (octant), then to South before hiding | installed-then-ejected tank's pre-hide facing sequence matches | sign of atan2 args is `(Δy=bld−unit, Δx=unit−bld)` + `0x7fff` offset — walk a fixture (`feedback_direction_bugs`) |
| State-2 diagonal track 0x43–0x46 by octant (target=building coord, no offset) | asm `0x0045912d`–`0x004591af` | track 0x47 present; 0x43–0x46 unconfirmed in `drive_track.rs` | `drive_track.rs`, install machine | force-track the unit onto the building coord via the octant-selected diagonal entry curve | sub-cell entry step plays before walls-up | confirm tracks 0x43–0x46 point tables exist; do NOT reuse the exit `0x47` `±0x80` offset for the install |
| Entry cell = footprint via own move; `DockingOffset` unused | `0x0047c520` | n/a | `EnterBunker` command | issue the approach move to the bunker footprint; gate install on "unit on footprint + stopped" | unit drives onto NATBNK then installs | do NOT compute a dock offset cell |
| Blocker-shove: scatter footprint occupants, wait until clear | asm `0x00458f15`–`0x00459049` | missing | install state 0/1 | on a unit sitting on the 2×2 footprint at install, scatter it (reuse existing scatter), block install until clear | install onto an occupied footprint waits + shoves | reuse the existing scatter primitive; do NOT skip (user chose full fidelity) |
| Hide `+0x150` is a light hide (keeps coordinate, live object) | `0x004593A0` no-replace + combat surface | 7b uses full conceal+replace (documented divergence) | `install_bunker_link` / release helpers | 7b: full `conceal`+`remove_occupancy`, re-place on every exit (incl. sell/destroy at the building cell) — output-equivalent | unit reappears at the building cell on sell, at a nearby passable cell on deploy | **combat/render slice must revisit** to the live-hidden model so the tank can fire/draw |
| Handshake HELLO→CAN_ENTER→(0x15)→mission 0x14 | `RADIO_INFERRED…` §9.4 + lifecycle | missing | `radio/receive.rs` bunker branch | admission over the bus (HELLO then CanEnter), then start `BunkerRuntime` | enemy/non-bunkerable/occupied rejected at CanEnter | do NOT model 0x15 as a wire `RadioMessage` argument confusion — it triggers the building's bunker mission |

### Stale Docs / Follow-up

- Update `docs/plans/2026-06-02-tank-bunker-lifecycle-design.md` **Ledger item 11** and the "Open RE gaps" section: the install inter-state waits are **facing-turn completions on `unit+0x388`, not timer durations** (no magic constants); `0x8000` is desired facing **South**; the install force-tracks are the **diagonal entry curves 0x43–0x46** (octant-selected), distinct from the exit `0x47`; `DockingOffset` is confirmed unused. The design's `BunkerRuntime.timer: MissionTimer` for the install waits should be replaced by reading the unit's facing-turn completion (or kept only as a safety cap).
- `+0x214` reader and the COL-verified `+0x150`/`+0x480` decompile remain the only deferred items; neither blocks 7b (the hide model is decided + the behaviors are confirmed from call shape).

## Sources

- Ghidra decompiled: `0x00458E50` (decompile + full disassemble), `0x004c9220`, `0x004c9480`, `0x004c93d0`, `0x0047c520`, `0x004595C0`, `0x004593A0`, `0x0070FB50`; callers(`0x00458E50`)=`0x0044B780`; callees(`0x00458E50`).
- Ghidra RTTI: TypeDescriptor `.?AVUnitClass@@` @ `0x00842d88` (base `0x00842d80`); COL `pTypeDescriptor` @ `0x0080cbf8` (COL base `0x0080cbec`); COL→vtable backlink not analyzed in this DB.
- Prior docs (cited, not redone): `TANK_BUNKER_ENTRY_EXIT_VISIBLE_LIFECYCLE`, `BUNKER_0X2E4_LIFECYCLE_EXIT_CLEAR_PATH`, `BUNKER_SERVICEDEPOT_0X2E4_RECIPROCAL_LINK_TEARDOWN`, `NATBNK_BUNKER_0X2E4_ROW_HELPER_PATH`, `CHRONO_MINER_FORCE_TRACK_0X47_REFINERY_EXIT`, `RADIO_INFERRED_CODES_AND_DOCK_HANDSHAKE_ORDER`.
- INI: `ini/rulesmd.ini:719/720/13722-13751`, `ini/artmd.ini` `[NATBNK]`.
