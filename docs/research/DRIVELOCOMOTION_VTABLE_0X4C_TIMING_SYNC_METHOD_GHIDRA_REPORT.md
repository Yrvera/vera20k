# DriveLocomotion vtable +0x4C TIMING_SYNC Method -- Ghidra Research Report

**Address(es):** `0x004B0EF0` primary method; dispatch from `UnitClass::Receive_Radio @ 0x00737430`; DriveLocomotion ILocomotion vtable `0x007E7EB0`, slot `+0x4C` at `0x007E7EFC`.
**Investigation Mode:** exhaustive-slice
**Claimed Scope:** The exact DriveLocomotion ILocomotion `+0x4C` method called by UnitClass radio `0x16` / TIMING_SYNC during refinery docking, its arguments, touched timer fields, and the visible dock-timing consequence for chrono miner / harvester unloading.
**Non-Scope:** Full DriveLocomotion track processing, all locomotor vtables, full harvester radio state machine, and the exact semantic name of every `TechnoClass+0x388` consumer outside this dock-sync path.
**Confidence:** High for address, arguments, writes, and dock gate; Medium for human-readable semantic label because prior docs disagree between "body/primary/turret facing" names, while the binary evidence only requires "owner +0x388 FacingClass/RateTimer".
**Active in YR:** Yes. Standard YR harvester refinery docking reaches `UnitClass::Receive_Radio` case `0x16`; `[CMIN]`, `[CMINWO]`, `[HARV]`, and `[HORV]` are `Harvester=yes` with `Dock=NAREFN,GAREFN`, and `[GAREFN]` / `[NAREFN]` have `DockUnload=yes` in `ini/rulesmd.ini`.

## 1. Overview

`UnitClass::Receive_Radio` case `0x16` is the receiver-side TIMING_SYNC handler used after the building-side CAN_DOCK acceptance sequence. For a unit whose active locomotor is DriveLocomotion, the indirect `ILocomotion+0x4C` call resolves to `DriveLocomotionClass__Do_Turn @ 0x004B0EF0`.

The method does not implement path speed or a crawl-rate accumulator inside `DriveLocomotionClass`. It is a wrapper that selects the linked owner object from `DriveLocomotion+0x08`, adds `0x388`, and calls `RateTimer__Set @ 0x004C9220` with the caller-provided target, here low word `0x4000`. The same owner `+0x388` timer is sampled by both radio `0x16` and `Mission_Deploy_Building`; the dump loop waits until this timer is within the `0x4000` quantization window before ore unloading begins.

## 2. Class Layout / Key Offsets

| Field | Offset | Type / role | Evidence | Active in YR |
|---|---:|---|---|---|
| Unit active ILocomotion pointer | `UnitClass+0x674` (`param_1[0x19D]`) | COM ILocomotion pointer used by radio `0x16` dispatch | `UnitClass::Receive_Radio @ 0x00737430`, case `0x16` loads `[ESI+0x674]` before `call [vtable+0x4C]` | Yes; active for UnitClass harvesters |
| Chrono teleporting gate | `UnitClass+0x6AF` | Nonzero skips TIMING_SYNC timer set | `UnitClass::Receive_Radio @ 0x00737430`, case `0x16`; `Mission_Deploy_Building @ 0x0073D630` same guard before reissuing `0x4000` | Conditional; active gate, normally false during refinery drive-in |
| Drive linked owner | `DriveLocomotion+0x08` | Pointer to owning Foot/Techno object | `0x004B0EF0` loads first stack arg, reads `[arg+0x08]`, then adds `0x388` | Yes for DriveLocomotion-bound units |
| Owner dock-sync timer | `Owner+0x388` | FacingClass / RateTimer object sampled and written by this path | `0x007376C8` and `0x0073DF43` use `LEA ECX,[ESI+0x388]` before `RateTimer__Current`; `0x004B0EF0` uses `[Drive+0x08]+0x388` before `RateTimer__Set` | Yes; sampled in dock radio and unload mission |
| RateTimer target low word | `timer+0x00` low 16 bits | Target/current-facing-like value compared against `0x4000` | `RateTimer__Set @ 0x004C9220`, `RateTimer__Current @ 0x004C93D0` | Yes |
| RateTimer previous/interpolation value | `timer+0x04` | Retarget baseline copied from current interpolated value before writing a new target | `RateTimer__Set @ 0x004C9220` writes `*(timer+0x04)` before `*(timer+0x00)` | Yes |
| RateTimer start frame | `timer+0x08` | Set to `g_CurrentFrameCounter` on retarget | `RateTimer__Set @ 0x004C9220` | Yes |
| RateTimer duration | `timer+0x10` | `abs(delta) / rate` integer duration | `RateTimer__Set @ 0x004C9220`; `RateTimer__Current @ 0x004C93D0` | Yes |
| RateTimer rate | `timer+0x14` | Rotation/interpolation rate; `rate < 1` snaps | `FUN_004C91E0`, `FUN_004C91C0`, `FUN_004C9680`, `RateTimer__Set`, `RateTimer__Current` | Yes |

## 3. Core Logic

### 3.1 Dispatch Site -- UnitClass::Receive_Radio case `0x16`

Verified at `0x00737430`, case `0x16`.

1. Calls `FootClass__Receive_Radio(sender, 0x16, payload)` first.
2. If `Unit+0x6AF == 0`, reads `RateTimer__Current(Unit+0x388, out)`.
3. If the returned low word is not `0x4000`, it prepares a target whose low word is `0x4000`, loads `Unit+0x674`, and calls the active locomotor vtable slot `+0x4C`.
4. Returns `1` immediately after setting the timer target.
5. If the timer already reads `0x4000`, it tests locomotor `Is_Moving`, destination, destination type building (`WhatAmI == 6`), and unit mission `7`; if all match, it sends radio `0x15` back to the destination building.

**Active in YR:** Yes. This is the standard UnitClass radio handler and is reached by refinery docking for harvesters. Evidence: `0x00737430`; prior sender report `RADIO_0x16_SENDER_BUILDINGCLASS_CASE_0x0E_GHIDRA_REPORT.md`.

### 3.2 Vtable Resolution

DriveLocomotion ILocomotion vtable bytes at `0x007E7EB0` resolve slot `+0x4C` (`0x007E7EFC`) to `0x004B0EF0`. Ghidra xrefs to `0x004B0EF0` show only the data xref from `0x007E7EFC`, matching a vtable-only virtual method.

The vtable slot is therefore:

| Vtable | Slot | Entry address | Function label | Verified behavior |
|---|---:|---:|---|---|
| DriveLocomotion ILocomotion | `+0x4C` | `0x004B0EF0` | `DriveLocomotionClass__Do_Turn` | Wrapper around `RateTimer__Set(owner+0x388, &target)` |

**Active in YR:** Yes for DriveLocomotion units. Chrono Miner uses drive behavior during the refinery dock approach phase when it is not mid-teleport. Evidence: Unit case `0x16` only gates out `Unit+0x6AF != 0`; CMIN rules mark it as harvester with refinery docks.

### 3.3 Method Semantics -- `0x004B0EF0`

`DriveLocomotionClass__Do_Turn` takes two stack arguments in this call shape:

| Argument | Value in TIMING_SYNC call | Use |
|---|---|---|
| arg1 | active DriveLocomotion ILocomotion pointer (`Unit+0x674`) | Reads linked owner at `arg1+0x08` |
| arg2 | dword target; low word explicitly `0x4000` | Address of this stack dword is passed to `RateTimer__Set` |

The method performs:

1. Reads owner pointer from `DriveLocomotion+0x08`.
2. Computes `owner + 0x388`.
3. Passes a pointer to the caller's target dword to `RateTimer__Set`.
4. Returns without reading path, track, speed, terrain, or dock fields.

**Active in YR:** Yes when the active locomotor vtable is DriveLocomotion. Evidence: `read_memory 0x007E7EB0` and decompile/byte-read of `0x004B0EF0`.

### 3.4 RateTimer Effects

`RateTimer__Set @ 0x004C9220` is the real writer. Important details:

- If the current target low word already equals the new target low word, it returns `0` and does not retarget.
- If `rate > 0`, it first computes the current interpolated value from the old target/start frame/duration and stores that as the new baseline at `timer+0x04`; this prevents a visible snap when retargeting mid-turn.
- It then writes the new 4-byte target to `timer+0x00`.
- If `rate > 0`, it writes `g_CurrentFrameCounter` to `timer+0x08` and writes duration `abs(new_low - baseline_low) / rate` to `timer+0x10` using integer division.
- If `rate < 1`, `RateTimer__Current` snaps to the target and `RateTimer__Set` does not start a meaningful interpolation window.

**Active in YR:** Yes. `RateTimer__Set` and `RateTimer__Current` are shared active timing primitives; this specific call is reached from active harvester dock logic. Evidence: `0x004C9220`, `0x004C93D0`, `0x007376C8`, `0x004B0EF0`.

### 3.5 Unload Mission Gate

`UnitClass::Mission_Deploy_Building @ 0x0073D630` repeats the same owner `+0x388` timer check during refinery unload startup:

1. `RateTimer__Current(Unit+0x388, out)`.
2. Compute `((current >> 7) + 1) & 0x1FE`.
3. If the result is not `0x80`, then if `Unit+0x6AF == 0`, call locomotor `+0x4C` again with low word `0x4000`, and return mission delay `5`.
4. Only once the expression equals `0x80` does the mission proceed into unload state setup (`Unit+0x6D1`, counters/timers, phase changes).

The accepted low-word window is `current >> 7` equal to `0x7F` or `0x80`, i.e. current in the inclusive range `0x3F80..0x407F`. Exact `0x4000` is the target, but unload startup uses a quantized two-bucket window rather than exact equality.

**Active in YR:** Yes. This is the standard harvester unload mission for `[GAREFN]` / `[NAREFN]` DockUnload paths. Evidence: `0x0073D630`, bytes around `0x0073DF43..0x0073DFAC`.

## 4. INI Keys

No INI key names or values are read by `0x004B0EF0`; it only touches the owner timer. These INI rows establish that the caller path is active in stock YR:

| INI path | Value | Effect in this slice | Active in YR |
|---|---|---|---|
| `ini/rulesmd.ini:[CMIN] Dock=NAREFN,GAREFN` | refinery dock list | Chrono Miner returns to standard Allied/Soviet refineries | Yes |
| `ini/rulesmd.ini:[CMIN] Harvester=yes` | harvester unit | Enables harvester dock/unload behavior | Yes |
| `ini/rulesmd.ini:[CMIN] Speed=4` | normal movement speed | Not read by TIMING_SYNC method; relevant only to approach before this timer gate | Yes |
| `ini/rulesmd.ini:[GAREFN] DockUnload=yes` | refinery unload building | Building-side dock path sends `0x16` after move/enter sequence | Yes |
| `ini/rulesmd.ini:[NAREFN] DockUnload=yes` | refinery unload building | Same for Soviet refinery | Yes |

## 5. Integration Points

| Integration point | Finding | Active in YR |
|---|---|---|
| Sender | BuildingClass CAN_DOCK case sends `0x16` after `0x12` returns `0x14` and after `0x18` | Yes; `RADIO_0x16...` and `BuildingClass::Receive_Radio @ 0x0043C2D0` |
| Receiver | UnitClass case `0x16` samples `Unit+0x388`; dispatches active locomotor `+0x4C` with low word `0x4000` if not already there | Yes; `0x00737430` |
| Concrete Drive method | DriveLocomotion slot `+0x4C` is `0x004B0EF0`, wrapper to `RateTimer__Set(owner+0x388)` | Yes; `0x007E7EFC -> 0x004B0EF0` |
| Dump startup | `Mission_Deploy_Building` repeats `+0x4C(0x4000)` until `Unit+0x388` quantizes to `0x80` | Yes; `0x0073D630` |
| Player-visible effect | The miner does not begin the ore dump state until the owner `+0x388` timer reaches the `0x4000` heading/timer window; changing this can shift the dock-arrival pause/pivot cadence before unloading | Yes; visible every unload cycle, especially if timer is not already near `0x4000` |

## 6. Current Rust Implementation Status

The Rust code was scanned only for context; no Rust files were modified. Codegraph found the miner docking implementation around `src/sim/miner/miner_dock_sequence.rs` and `src/sim/miner/miner_system.rs`. This report does not audit that implementation in detail because the slot scope is binary method resolution, but a future parity task should check whether Rust models:

- a per-unit 16-bit facing/timer field equivalent to `Unit+0x388`,
- radio/dock TIMING_SYNC retarget to `0x4000`,
- the unload-start wait window `0x3F80..0x407F`,
- and the `Unit+0x6AF` chrono-in-progress gate.

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| `UnitClass::Receive_Radio` case `0x16` dispatch | verified | `0x00737430`, bytes around `0x007376AD..0x0073770F` | none |
| DriveLocomotion vtable slot `+0x4C` | verified | `read_memory 0x007E7EB0`, slot `0x007E7EFC -> 0x004B0EF0` | none |
| `DriveLocomotionClass__Do_Turn @ 0x004B0EF0` | verified | decompile and byte-read of `0x004B0EF0`; callee `RateTimer__Set` | none |
| `RateTimer__Set @ 0x004C9220` | verified | decompile of `0x004C9220` | none for this slice |
| `RateTimer__Current @ 0x004C93D0` | verified | decompile of `0x004C93D0`; callers at `0x007376C8` and `0x0073DF43` | none for this slice |
| `Mission_Deploy_Building` dock wait gate | verified | `0x0073D630`, bytes around `0x0073DF43..0x0073DFAC` | none for this slice |
| Human-readable name for `owner+0x388` | touched-not-exhausted | multiple docs label this timer differently; this slice verifies offset and operations, not global naming | full Techno/Unit FacingClass naming reconciliation |
| WalkLocomotion slot `+0x4C` comparison | deferred | out of requested DriveLocomotion scope; prior report says `0x0075AE00` | separate locomotor-vtable audit if needed |

## 8. Open Questions - Final State

- `[RESOLVED] OQ01 - Which exact DriveLocomotion method is slot +0x4C?` Answer: vtable base `0x007E7EB0`, slot `0x007E7EFC`, entry `0x004B0EF0` (`DriveLocomotionClass__Do_Turn`). Evidence: Ghidra `read_memory 0x007E7EB0`; xref to `0x004B0EF0` from `0x007E7EFC`.
- `[RESOLVED] OQ02 - What arguments are passed by TIMING_SYNC?` Answer: Unit passes active locomotor pointer from `Unit+0x674` plus a target dword whose low word is explicitly `0x4000`. Evidence: `UnitClass::Receive_Radio @ 0x00737430`, bytes around `0x007376D8..0x007376F5`.
- `[RESOLVED] OQ03 - What does DriveLocomotion slot +0x4C write/read?` Answer: It reads `DriveLocomotion+0x08`, computes owner `+0x388`, and lets `RateTimer__Set` write the timer fields (`+0x00`, `+0x04`, `+0x08`, `+0x10`) while reading rate at `+0x14`. Evidence: `0x004B0EF0`, `0x004C9220`.
- `[RESOLVED] OQ04 - Is 0x4000 a speed/crawl-rate target?` Answer: Not in this method. In DriveLocomotion `+0x4C`, it is a FacingClass/RateTimer target for owner `+0x388`; the player-visible timing effect is that unload startup waits for this timer to reach the `0x4000` quantization window. Evidence: `0x004B0EF0`; `0x0073DF43..0x0073DFAC`.
- `[RESOLVED] OQ05 - Is the path active in standard YR, not TS legacy?` Answer: Yes. No TS-only flag gates the dispatch; standard YR CMIN/HARV/HORV definitions are harvesters docked to GAREFN/NAREFN, and those refineries have `DockUnload=yes`. Evidence: `ini/rulesmd.ini` lines for `[CMIN]`, `[HARV]`, `[HORV]`, `[GAREFN]`, `[NAREFN]`; `0x0043C2D0` sender report.
- `[DEFERRED] OQ06 - What global canonical name should `Unit+0x388` use across all docs?` Category: out-of-scope. Reason: this slice only needs the offset and operations; existing docs use body/primary/turret/facing names inconsistently. Next step: dedicated TechnoClass/UnitClass FacingClass layout audit.

## Sources

- Ghidra `read_memory 0x007E7EB0` (DriveLocomotion ILocomotion vtable)
- Ghidra decompiled `UnitClass::Receive_Radio @ 0x00737430`
- Ghidra byte-read of case `0x16` around `0x007376A0`
- Ghidra decompiled and byte-read `DriveLocomotionClass__Do_Turn @ 0x004B0EF0`
- Ghidra decompiled `RateTimer__Set @ 0x004C9220`
- Ghidra decompiled `RateTimer__Current @ 0x004C93D0`
- Ghidra decompiled and byte-read `UnitClass::Mission_Deploy_Building @ 0x0073D630`, especially `0x0073DF43..0x0073DFAC`
- `C:/Users/enok/Documents/ra2-rust-game-docs/RADIO_0x16_SENDER_BUILDINGCLASS_CASE_0x0E_GHIDRA_REPORT.md`
- `C:/Users/enok/Documents/ra2-rust-game-docs/UNITCLASS_RECEIVE_RADIO_FULL_SWITCH_GHIDRA_REPORT.md`
- `C:/Users/enok/Documents/ra2-rust-game-docs/TECHNOCLASS_EXPANDED_STRUCT_LAYOUT.md`
- `ini/rulesmd.ini`

## Status

COMPLETE for the requested DriveLocomotion vtable `+0x4C` TIMING_SYNC method slice.
